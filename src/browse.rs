//! The list of files of the window that opens a file: the folder being looked at, its sub-folders first and then the files of the kinds the program reads. It only reads folders, so it is
//! tested without a screen.

use std::path::{Path, PathBuf};

/// A folder or a file in the list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
}

/// Whether a file name ends in one of the extensions (compared without regard to case); with no extensions in the list every name matches.
pub fn matches_extension(name: &str, extensions: &[&str]) -> bool {
    if extensions.is_empty() {
        return true;
    }
    match name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() => {
            extensions.iter().any(|e| e.eq_ignore_ascii_case(ext))
        }
        _ => false,
    }
}

/// The contents of a folder: the sub-folders first, then the files that match, each group in alphabetical order. Names that start with a dot are left out.
pub fn list(dir: &Path, extensions: &[&str]) -> Result<Vec<Entry>, String> {
    let read = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut folders = Vec::new();
    let mut files = Vec::new();
    for item in read.flatten() {
        let name = item.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let Ok(meta) = item.metadata() else {
            continue;
        };
        if meta.is_dir() {
            folders.push(Entry {
                name,
                is_dir: true,
                size: 0,
            });
        } else if meta.is_file() && matches_extension(&name, extensions) {
            files.push(Entry {
                name,
                is_dir: false,
                size: meta.len(),
            });
        }
    }
    folders.sort_by_key(|e| e.name.to_lowercase());
    files.sort_by_key(|e| e.name.to_lowercase());
    folders.extend(files);
    Ok(folders)
}

/// The folder where the window starts: the current one, or the one of the person when that cannot be known.
pub fn start_dir() -> PathBuf {
    std::env::current_dir()
        .ok()
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// A size for the list: bytes, KiB or MiB.
pub fn size_text(bytes: u64) -> String {
    if bytes >= 1_048_576 {
        format!("{:.1} MiB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1} KiB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

/// The state of the window that opens a file.
#[derive(Debug, Clone)]
pub struct Browser {
    pub dir: PathBuf,
    pub entries: Vec<Entry>,
    pub error: Option<String>,
    /// The text of the box where a path can be typed.
    pub path_text: String,
    /// The extensions shown; every file is shown when `show_all` is on.
    pub extensions: Vec<&'static str>,
    pub show_all: bool,
}

impl Browser {
    pub fn new(dir: PathBuf, extensions: &[&'static str]) -> Browser {
        let mut b = Browser {
            dir: PathBuf::new(),
            entries: Vec::new(),
            error: None,
            path_text: String::new(),
            extensions: extensions.to_vec(),
            show_all: false,
        };
        b.go(dir);
        b
    }

    fn filter(&self) -> &[&'static str] {
        if self.show_all {
            &[]
        } else {
            &self.extensions
        }
    }

    /// Reads the folder again (after the filter changed).
    pub fn refresh(&mut self) {
        match list(&self.dir, self.filter()) {
            Ok(entries) => {
                self.entries = entries;
                self.error = None;
            }
            Err(e) => {
                self.entries.clear();
                self.error = Some(e);
            }
        }
    }

    /// Goes to a folder. When it cannot be read the list is empty and the error is kept for the window to show.
    pub fn go(&mut self, dir: PathBuf) {
        self.dir = dir;
        self.path_text = self.dir.display().to_string();
        self.refresh();
    }

    /// Goes to the folder above, if there is one.
    pub fn up(&mut self) {
        if let Some(parent) = self.dir.parent() {
            if !parent.as_os_str().is_empty() {
                let parent = parent.to_path_buf();
                self.go(parent);
            }
        }
    }

    /// What a click on a name does: a folder is entered (and nothing is returned); a file is returned with its whole path.
    pub fn enter(&mut self, name: &str) -> Option<PathBuf> {
        let path = self.dir.join(name);
        if path.is_dir() {
            self.go(path);
            None
        } else {
            Some(path)
        }
    }

    /// What the path typed in the box means: a folder is entered, a file is returned.
    pub fn go_typed(&mut self) -> Option<PathBuf> {
        let path = PathBuf::from(self.path_text.trim());
        if path.is_dir() {
            self.go(path);
            None
        } else if path.is_file() {
            Some(path)
        } else {
            self.error = Some(format!("{}: not found", path.display()));
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// A folder of its own for each test, removed at the end.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(tag: &str) -> Scratch {
            let dir =
                std::env::temp_dir().join(format!("eh3d-browse-test-{}-{tag}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            Scratch(dir)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn extensions_are_compared_without_case() {
        assert!(matches_extension("TOP.GTL", &["gtl"]));
        assert!(matches_extension("top.gtl", &["GTL", "gbl"]));
        assert!(!matches_extension("top.txt", &["gtl"]));
        assert!(!matches_extension("gtl", &["gtl"]));
        assert!(!matches_extension(".gtl", &["gtl"]));
        assert!(matches_extension("anything", &[]));
    }

    #[test]
    fn folders_come_first_then_the_files_that_match_in_order() {
        let s = Scratch::new("order");
        fs::create_dir(s.0.join("zeta")).unwrap();
        fs::create_dir(s.0.join("Alpha")).unwrap();
        fs::write(s.0.join("b.gtl"), "x").unwrap();
        fs::write(s.0.join("A.gbl"), "x").unwrap();
        fs::write(s.0.join("notes.txt"), "x").unwrap();
        fs::write(s.0.join(".hidden.gtl"), "x").unwrap();
        let names: Vec<String> = list(&s.0, &["gtl", "gbl"])
            .unwrap()
            .into_iter()
            .map(|e| e.name)
            .collect();
        assert_eq!(names, ["Alpha", "zeta", "A.gbl", "b.gtl"]);
    }

    #[test]
    fn a_folder_that_does_not_exist_is_an_error_with_its_name() {
        let e = list(Path::new("/this/folder/does/not/exist"), &[]).unwrap_err();
        assert!(e.contains("exist"));
    }

    #[test]
    fn clicking_a_folder_enters_it_and_clicking_a_file_returns_it() {
        let s = Scratch::new("enter");
        fs::create_dir(s.0.join("sub")).unwrap();
        fs::write(s.0.join("sub").join("a.gtl"), "x").unwrap();
        let mut b = Browser::new(s.0.clone(), &["gtl"]);
        assert_eq!(b.enter("sub"), None);
        assert_eq!(b.dir, s.0.join("sub"));
        assert_eq!(b.entries.len(), 1);
        assert_eq!(b.enter("a.gtl"), Some(s.0.join("sub").join("a.gtl")));
        b.up();
        assert_eq!(b.dir, s.0);
    }

    #[test]
    fn show_all_lists_the_files_of_every_kind() {
        let s = Scratch::new("all");
        fs::write(s.0.join("a.gtl"), "x").unwrap();
        fs::write(s.0.join("b.txt"), "x").unwrap();
        let mut b = Browser::new(s.0.clone(), &["gtl"]);
        assert_eq!(b.entries.len(), 1);
        b.show_all = true;
        b.refresh();
        assert_eq!(b.entries.len(), 2);
    }

    #[test]
    fn a_typed_path_enters_a_folder_returns_a_file_or_complains() {
        let s = Scratch::new("typed");
        fs::write(s.0.join("a.gtl"), "x").unwrap();
        let mut b = Browser::new(s.0.clone(), &[]);
        b.path_text = s.0.join("a.gtl").display().to_string();
        assert_eq!(b.go_typed(), Some(s.0.join("a.gtl")));
        b.path_text = s.0.join("missing.gtl").display().to_string();
        assert_eq!(b.go_typed(), None);
        assert!(b.error.is_some());
        b.path_text = s.0.display().to_string();
        assert_eq!(b.go_typed(), None);
        assert!(b.error.is_none());
    }

    #[test]
    fn sizes_are_written_in_the_unit_that_reads_best() {
        assert_eq!(size_text(12), "12 B");
        assert_eq!(size_text(2048), "2.0 KiB");
        assert_eq!(size_text(3 * 1_048_576), "3.0 MiB");
    }
}
