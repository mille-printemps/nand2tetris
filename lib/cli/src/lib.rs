// The command-line shell shared by the nand2tetris tools.

use collections::deque::Deque;
use functional::functor::Functor;
use functional::io::IO;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

// The recursive persistent structures in collections nest deeply enough to overflow
// the default 8MB stack on realistic inputs.
const STACK_SIZE: usize = 16 * 1024 * 1024;

// Runs task on a thread with a stack large enough for the deep recursion in collections,
// and exits non-zero if it panics.
pub fn run_on_large_stack<F>(task: F)
where
    F: FnOnce() + Send + 'static,
{
    let worker = std::thread::Builder::new()
        .stack_size(STACK_SIZE)
        .spawn(task)
        .expect("failed to spawn the worker thread");

    if worker.join().is_err() {
        // The panic has already been reported by the default hook
        // adding a second panic here would only bury it.
        process::exit(1);
    }
}

// Whether the argument named a file or a folder.
// vm needs the distinction: a folder is a whole program and gets bootstrap code,
// a lone file is a fragment and does not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    File,
    Folder,
}

// A command-line argument resolved against the shared contract.
pub struct Source<D> {
    pub kind: SourceKind,
    // The folder the input came from — where output belongs.
    // "." when the argument carried no path.
    pub dir: PathBuf,
    // Stem for a tool that emits one combined file: the folder's name for a folder input,
    // the file's stem for a file input.
    pub stem: String,
    // The input files.
    pub files: D,
}

impl<D> Source<D> {
    // Path of the single combined output file, e.g. Pong/Pong.asm.
    pub fn combined_output(&self, extension: &str) -> String {
        self.dir
            .join(format!("{}.{}", self.stem, extension))
            .to_string_lossy()
            .into_owned()
    }
}

#[derive(Debug)]
pub enum SourceError {
    MissingArgument,
    WrongExtension { path: String, expected: String },
    NotFound(String),
    NoSourceFiles { dir: String, expected: String },
}

impl fmt::Display for SourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SourceError::MissingArgument => write!(formatter, "no input given"),
            SourceError::WrongExtension { path, expected } => {
                write!(formatter, "{} is not a .{} file", path, expected)
            }
            SourceError::NotFound(path) => write!(formatter, "{} does not exist", path),
            SourceError::NoSourceFiles { dir, expected } => {
                write!(formatter, "{} holds no .{} files", dir, expected)
            }
        }
    }
}

// Reports error with a usage line and exits non-zero.
pub fn abort(error: SourceError, usage: &str) -> ! {
    let program = program_name();
    eprintln!("{}: {}", program, error);
    eprintln!("Usage: {} {}", program, usage);
    process::exit(1);
}

// The running program's name, without directory or extension.
pub fn program_name() -> String {
    std::env::args()
        .next()
        .and_then(|path| {
            PathBuf::from(path)
                .file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "tool".to_string())
}

// The single argument, if one was given.
pub fn argument() -> Option<String> {
    std::env::args().nth(1)
}

fn has_extension(path: &Path, extension: &str) -> bool {
    path.extension().and_then(|found| found.to_str()) == Some(extension)
}

// The folder a path lives in. A bare file name yields ".".
// which is what "if no path is specified, operate on the current folder" means.
fn folder_of(path: &Path) -> PathBuf {
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
        _ => PathBuf::from("."),
    }
}

// A folder's own name. "." and ".." carry none,
// so fall back to resolving them against the filesystem.
fn folder_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .or_else(|| {
            fs::canonicalize(path).ok().and_then(|absolute| {
                absolute
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
            })
        })
        .unwrap_or_else(|| "Main".to_string())
}

fn files_in<D: Deque<String>>(dir: &Path, extension: &str) -> D {
    fs::read_dir(dir).map_or(D::empty(), |entries| {
        entries.flatten().fold(D::empty(), |files, entry| {
            let path = entry.path();
            if path.is_file() && has_extension(&path, extension) {
                files.push_back(path.to_string_lossy().into_owned())
            } else {
                files
            }
        })
    })
}

fn resolve_single_file<D: Deque<String>>(
    argument: &str,
    extension: &str,
) -> Result<Source<D>, SourceError> {
    let path = PathBuf::from(argument);

    if !has_extension(&path, extension) {
        return Err(SourceError::WrongExtension {
            path: argument.to_string(),
            expected: extension.to_string(),
        });
    }
    if !path.is_file() {
        return Err(SourceError::NotFound(argument.to_string()));
    }

    let stem = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Main".to_string());

    Ok(Source {
        kind: SourceKind::File,
        dir: folder_of(&path),
        stem,
        files: D::empty().push_back(argument.to_string()),
    })
}

// Resolves an argument naming a single file
pub fn resolve_file<D: Deque<String>>(
    argument: Option<&str>,
    extension: &str,
) -> Result<Source<D>, SourceError> {
    let argument = argument.ok_or(SourceError::MissingArgument)?;
    resolve_single_file(argument, extension)
}

// Resolves an argument naming a file or a folder
// the contract shared by vm, analyzer and compiler
pub fn resolve<D: Deque<String>>(
    argument: Option<&str>,
    extension: &str,
) -> Result<Source<D>, SourceError> {
    let argument = argument.ok_or(SourceError::MissingArgument)?;
    let path = PathBuf::from(argument);

    if !path.is_dir() {
        return resolve_single_file(argument, extension);
    }

    let files: D = files_in(&path, extension);
    if files.is_empty() {
        return Err(SourceError::NoSourceFiles {
            dir: argument.to_string(),
            expected: extension.to_string(),
        });
    }

    Ok(Source {
        kind: SourceKind::Folder,
        stem: folder_name(&path),
        dir: path,
        files,
    })
}

// Output path for a tool that emits one file per input: same folder,
// the input's stem, then suffix (e.g. "T.xml" or ".vm").
pub fn sibling_path(input: &str, suffix: &str) -> String {
    let path = PathBuf::from(input);
    let stem = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    folder_of(&path)
        .join(format!("{}{}", stem, suffix))
        .to_string_lossy()
        .into_owned()
}

// Reads input, applies transform, writes the result to output.
pub fn process_file<F>(input: String, output: String, transform: F)
where
    F: FnOnce(String) -> Result<String, String> + 'static,
{
    IO::<String>::read_file(input)
        .flat_map(move |content: String| match transform(content) {
            Ok(result) => IO::<String>::write_file(output, result),
            Err(error) => IO::Error(error),
        })
        .unsafe_run()
        .unwrap_or_else(|error| panic!("Error: {}", error));
}

// Joins a deque of lines with newlines — the shape every tool's output takes.
pub fn join_lines<D: Deque<String>>(lines: &D) -> String {
    lines
        .iter()
        .map(|line| (*line).clone())
        .collect::<Vec<String>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use collections::deque::BankersDeque;
    use collections::Empty;

    type Paths = BankersDeque<String>;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cli-tests-{}", name));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("failed to create the scratch folder");
        dir
    }

    fn write(path: &Path, contents: &str) {
        fs::write(path, contents).expect("failed to write a fixture");
    }

    fn collect(files: &Paths) -> Vec<String> {
        files.iter().map(|path| (*path).clone()).collect()
    }

    #[test]
    fn missing_argument_is_an_error() {
        let resolved = resolve::<Paths>(None, "vm");
        assert!(matches!(resolved, Err(SourceError::MissingArgument)));
    }

    #[test]
    fn extension_is_mandatory() {
        let dir = scratch("extension");
        let path = dir.join("Prog.txt");
        write(&path, "");
        let resolved = resolve::<Paths>(Some(path.to_str().unwrap()), "vm");
        assert!(matches!(resolved, Err(SourceError::WrongExtension { .. })));
    }

    #[test]
    fn missing_file_is_reported_not_panicked() {
        let resolved = resolve::<Paths>(Some("/nonexistent/Prog.vm"), "vm");
        assert!(matches!(resolved, Err(SourceError::NotFound(_))));
    }

    #[test]
    fn empty_folder_is_an_error() {
        let dir = scratch("empty");
        let resolved = resolve::<Paths>(Some(dir.to_str().unwrap()), "vm");
        assert!(matches!(resolved, Err(SourceError::NoSourceFiles { .. })));
    }

    // The spec's "if no path is specified, operate on the current folder".
    #[test]
    fn bare_file_name_resolves_to_the_current_folder() {
        let dir = scratch("bare");
        write(&dir.join("Loose.vm"), "");
        let previous = std::env::current_dir().expect("no current folder");
        std::env::set_current_dir(&dir).expect("failed to enter the scratch folder");

        // Restore before asserting: a panic here would leave every other test
        // running in the wrong folder.
        let resolved = resolve::<Paths>(Some("Loose.vm"), "vm");
        std::env::set_current_dir(previous).expect("failed to restore the folder");

        let source = resolved.expect("should resolve");
        assert_eq!(source.dir, PathBuf::from("."));
        assert_eq!(source.stem, "Loose");
        assert_eq!(source.combined_output("asm"), "./Loose.asm");
    }

    // A lone file takes its own name, not its folder's.
    #[test]
    fn single_file_output_is_named_after_the_file() {
        let dir = scratch("naming");
        let folder = dir.join("MyFolder");
        fs::create_dir_all(&folder).expect("failed to create the folder");
        let path = folder.join("Bar.vm");
        write(&path, "");

        let source = resolve::<Paths>(Some(path.to_str().unwrap()), "vm").expect("should resolve");

        assert_eq!(source.kind, SourceKind::File);
        assert_eq!(source.stem, "Bar");
        assert_eq!(
            source.combined_output("asm"),
            folder.join("Bar.asm").to_string_lossy()
        );
    }

    // A folder takes the folder's name, and output lands inside it.
    #[test]
    fn folder_output_is_named_after_the_folder() {
        let dir = scratch("folder");
        let folder = dir.join("Pong");
        fs::create_dir_all(&folder).expect("failed to create the folder");
        write(&folder.join("Main.vm"), "");
        write(&folder.join("Pong.vm"), "");
        write(&folder.join("notes.txt"), "");

        let source =
            resolve::<Paths>(Some(folder.to_str().unwrap()), "vm").expect("should resolve");

        assert_eq!(source.kind, SourceKind::Folder);
        assert_eq!(source.stem, "Pong");
        assert_eq!(source.files.len(), 2, "only .vm files are collected");
        assert_eq!(
            source.combined_output("asm"),
            folder.join("Pong.asm").to_string_lossy()
        );
    }

    // A folder named "Foo.vm" must not be mistaken for a source file.
    #[test]
    fn folders_are_not_collected_as_files() {
        let dir = scratch("decoy");
        fs::create_dir_all(dir.join("Decoy.vm")).expect("failed to create the decoy");
        write(&dir.join("Real.vm"), "");

        let source = resolve::<Paths>(Some(dir.to_str().unwrap()), "vm").expect("should resolve");

        assert_eq!(collect(&source.files).len(), 1);
        assert!(collect(&source.files)[0].ends_with("Real.vm"));
    }

    #[test]
    fn asm_rejects_a_folder() {
        let dir = scratch("asm-folder");
        let resolved = resolve_file::<Paths>(Some(dir.to_str().unwrap()), "asm");
        assert!(matches!(resolved, Err(SourceError::WrongExtension { .. })));
    }

    #[test]
    fn output_lands_beside_the_input() {
        assert_eq!(
            sibling_path("../06/max/Max.asm", ".hack"),
            "../06/max/Max.hack"
        );
        assert_eq!(sibling_path("Main.jack", "T.xml"), "./MainT.xml");
        assert_eq!(sibling_path("a/b/Square.jack", ".vm"), "a/b/Square.vm");
    }

    #[test]
    fn join_lines_uses_newlines() {
        let lines = Paths::empty()
            .push_back("first".to_string())
            .push_back("second".to_string());
        assert_eq!(join_lines(&lines), "first\nsecond");
        assert_eq!(join_lines(&Paths::empty()), "");
    }
}
