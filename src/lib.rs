use std::{
    fs::{File, create_dir, create_dir_all, read_link, remove_dir, remove_dir_all, remove_file},
    io,
    path::Path,
};

use permitit::Permit;

macro_rules! iopermit {
    ($f:expr, $($ioe:ident),+ $(,)?) => {{
        use std::io::ErrorKind as IOE;
        let initial = $f;
        let mut f = initial;
        $(
            f = f.permit(|e| {
                tracing::debug!("Permitting {:?} for {:?}", IOE::$ioe, stringify!($f));
                e.kind() == IOE::$ioe
            });
        )+
        f
    }};
}

/// # Creates a directory.
/// Existing directories are ignored. Does not recurse.
pub fn mkdir<P>(dir: P) -> io::Result<()>
where
    P: AsRef<Path>,
{
    iopermit!(create_dir(dir), AlreadyExists)
}

/// # Creates a file.
/// Ignores attempts to create a file that already exists. Roughly corresponds to touch.
pub fn mkf<P>(file: P) -> io::Result<()>
where
    P: AsRef<Path>,
{
    iopermit!(File::create_new(file).map(drop), AlreadyExists)
}

/// # Creates a file, with parents.
/// Ignores attempts to create a file that already exists.
pub fn mkf_p<P>(file: P) -> io::Result<()>
where
    P: AsRef<Path>,
{
    if let Some(parent) = file.as_ref().parent() {
        // NOTE: This if prevents unnecessary logs
        if !parent.exists() {
            mkdir_p(parent)?
        }
    }

    iopermit!(File::create_new(file).map(drop), AlreadyExists)
}

/// # Creates a directory and all its parents.
/// Existing directores are ignored
pub fn mkdir_p<P>(dir: P) -> io::Result<()>
where
    P: AsRef<Path>,
{
    iopermit!(create_dir_all(dir), AlreadyExists)
}

/// # Removes a directory
/// Ignores attempts to remove missing or populated directories.
pub fn rmdir<P>(dir: P) -> io::Result<()>
where
    P: AsRef<Path>,
{
    iopermit!(remove_dir(dir), NotFound, DirectoryNotEmpty)
}

/// # Removes a directory recursively
/// Ignores attempts to remove missing directories.
pub fn rmdir_r<P>(dir: P) -> io::Result<()>
where
    P: AsRef<Path>,
{
    iopermit!(remove_dir_all(dir), NotFound)
}

/// # Removes a file or symlink.
/// Ignores attempts to remove missing files.
pub fn rmf<P>(file: P) -> io::Result<()>
where
    P: AsRef<Path>,
{
    iopermit!(remove_file(file), NotFound)
}

/// # Removes a path.
/// Removes a symlink, file, or directory, deciding which internally.
pub fn rm<P>(path: P) -> io::Result<()>
where
    P: AsRef<Path>,
{
    let p = path.as_ref();
    if p.is_symlink() || p.is_file() {
        rmf(path)
    } else {
        rmdir(path)
    }
}

/// # Check whether a path is a directory.
/// Follows symlinks.
pub fn is_dir<P>(path: P) -> io::Result<bool>
where
    P: AsRef<Path>,
{
    let p = path.as_ref();
    Ok(p.is_dir() || (p.is_symlink() && read_link(path)?.is_dir()))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn usr_is_dir() {
        assert!(is_dir("/usr").is_ok())
    }

    #[test]
    fn mkf_with_parents() {
        let f = Path::new("/tmp/fshelpers/path/to/test/file");
        assert!(mkf_p(f).is_ok() && f.exists())
    }

    #[test]
    fn rm_file_ignore_missing() {
        let f = Path::new("/tmp/fshelpers/path/to/test/nonexistent");
        assert!(rm(f).is_ok() && !f.exists())
    }

    #[test]
    fn rmdir_ignore_populated() {
        let d = Path::new("/tmp/fshelpers/path/to/test");
        mkf_p(d.join("hello")).unwrap();
        assert!(rmdir(d).is_ok() && d.exists())
    }

    #[test]
    fn create_and_remove_dir() {
        assert!(mkdir("hello").is_ok() && rmdir("hello").is_ok())
    }

    #[test]
    fn create_dir_with_parents_and_ignore_remove() {
        let d = Path::new("hi/hello");
        assert!(mkdir_p("hi/hello").is_ok() && rmdir("hello").is_ok() && d.exists())
    }

    #[test]
    fn rm_recursive() {
        assert!(rmdir_r("/tmp/fshelpers").is_ok());
        assert!(rmdir_r("hi").is_ok());
    }
}
