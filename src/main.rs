use git2::{Oid, RebaseOperationType, Repository};
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::path::Path;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn write_file(path: &Path, text: &str) -> Result<()> {
    let mut file = File::create(path)?;
    file.write_all(text.as_bytes())?;
    Ok(())
}

fn commit(repo: &Repository, files: &[&str], message: &str) -> Result<Oid> {
    let tree_id = {
        let mut index = repo.index()?;
        for file in files {
            index.add_path(Path::new(file))?;
        }
        let tree_id = index.write_tree()?;
        index.write()?;
        tree_id
    };
    let tree = repo.find_tree(tree_id)?;

    let signature = repo.signature()?;

    let oid = if repo.is_empty()? {
        repo.commit(Some("HEAD"), &signature, &signature, message, &tree, &[])?
    } else {
        let commit = repo.find_reference("HEAD")?.peel_to_commit()?;
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &[&commit],
        )?
    };
    Ok(oid)
}

fn main() -> Result<()> {
    let repo_path = Path::new("tmp-repo");

    let repo = Repository::init(repo_path).unwrap();

    write_file(&repo_path.join("file1.txt"), "Hi\n")?;
    commit(&repo, &["file1.txt"], "Initial commit")?;

    repo.branch(
        "feature",
        &repo.find_reference("HEAD")?.peel_to_commit()?,
        false,
    )?;
    repo.set_head("refs/heads/feature")?;
    repo.checkout_head(None)?;

    write_file(&repo_path.join("file1.txt"), "Hi\nFeature\n")?;
    commit(&repo, &["file1.txt"], "First commit on feature")?;

    write_file(&repo_path.join("file1.txt"), "Hi\nFeature\nAgain\n")?;
    commit(&repo, &["file1.txt"], "Second commit on feature")?;

    let master = repo.find_annotated_commit(repo.refname_to_id("refs/heads/master")?)?;
    let head = repo.reference_to_annotated_commit(&repo.head()?)?;

    let mut rebase = repo.rebase(Some(&head), Some(&master), None, None)?;

    println!("Number of commits to rebase: {}", rebase.len());

    // Ok, at the time of writing this, there's no support for changing the operations (a la
    // interactive rebase). Even if you change the kind in the struct (in C) here, it doesn't
    // have any effect.
    // The reason is that this whole thing is not an interactive rebase, all it can do is pick.
    // See https://github.com/libgit2/libgit2/pull/2482#issuecomment-60630837
    // and https://github.com/libgit2/libgit2/issues/3795

    while let Some(operation) = rebase.next() {
        let operation = operation?;
        println!("Operation: {:?}", operation.kind());
        rebase.commit(None, &repo.signature()?, None)?;
    }

    rebase.finish(None)?;

    Ok(())
}
