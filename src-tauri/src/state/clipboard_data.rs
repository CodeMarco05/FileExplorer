use std::path::{Path, PathBuf};
use std::fs;
use std::sync::{Arc, Mutex};
use std::io;
use arboard::Clipboard as SystemClipboard;
use crate::log_info;
use crate::log_error;
use crate::log_warn;

#[derive(Clone, Debug)]
pub enum ClipboardContent {
    None,
    FilePath(PathBuf),
    FolderPath(PathBuf),
    TextContent(String),
    BinaryContent(Vec<u8>),
    MultipleItems(Vec<PathBuf>),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ClipboardOperation {
    None,
    Copy,
    Cut,
}

pub struct Clipboard {
    content: ClipboardContent,
    operation: ClipboardOperation,
    system_clipboard: Option<SystemClipboard>,
}

impl Default for Clipboard {
    fn default() -> Self {
        Self::new()
    }
}

impl Clipboard {
    pub fn new() -> Self {
        log_info!("Initializing clipboard");
        let system_clipboard = SystemClipboard::new().ok();
        if system_clipboard.is_none() {
            log_warn!("Failed to initialize system clipboard");
        }
        
        Clipboard {
            content: ClipboardContent::None,
            operation: ClipboardOperation::None,
            system_clipboard,
        }
    }
}

pub struct ClipboardState(pub Arc<Mutex<Clipboard>>);

impl ClipboardState {
    pub fn new() -> Self {
        log_info!("Creating new clipboard state");
        Self(Arc::new(Mutex::new(Clipboard::new())))
    }

    pub fn copy_path(&self, path: &Path) -> io::Result<()> {
        log_info!(format!("Copying path: {}", path.to_string_lossy()).as_str());
        let path_str = path.to_string_lossy().to_string();
        let mut clipboard = self.0.lock().unwrap();

        // Set in system clipboard
        if let Some(system_clipboard) = clipboard.system_clipboard.as_mut() {
            if let Err(err) = system_clipboard.set_text(path_str.clone()) {
                log_error!(format!("Failed to set path in system clipboard: {}", err).as_str());
            }
        }

        // Set in internal clipboard
        if path.is_file() {
            clipboard.content = ClipboardContent::FilePath(path.to_path_buf());
            log_info!("Copied file path to clipboard");
        } else if path.is_dir() {
            clipboard.content = ClipboardContent::FolderPath(path.to_path_buf());
            log_info!("Copied folder path to clipboard");
        }
        
        clipboard.operation = ClipboardOperation::Copy;
        Ok(())
    }

    pub fn copy_file_content(&self, path: &Path) -> io::Result<()> {
        log_info!(format!("Copying file content from: {}", path.to_string_lossy()).as_str());
        
        if !path.is_file() {
            let err = io::Error::new(io::ErrorKind::InvalidInput, "Not a file");
            log_error!(format!("Failed to copy file content: {}", err).as_str());
            return Err(err);
        }

        let content = match fs::read(path) {
            Ok(data) => data,
            Err(e) => {
                log_error!(format!("Failed to read file content: {}", e).as_str());
                return Err(e);
            }
        };
        
        let mut clipboard = self.0.lock().unwrap();

        // Try to set as text if it's valid UTF-8
        if let Ok(text) = String::from_utf8(content.clone()) {
            // Set in system clipboard
            if let Some(system_clipboard) = clipboard.system_clipboard.as_mut() {
                if let Err(err) = system_clipboard.set_text(text.clone()) {
                    log_error!(format!("Failed to set text in system clipboard: {}", err).as_str());
                }
            }
            clipboard.content = ClipboardContent::TextContent(text);
            log_info!("Copied text content to clipboard");
        } else {
            clipboard.content = ClipboardContent::BinaryContent(content);
            log_info!("Copied binary content to clipboard");
            // Binary can't be put in system clipboard directly
        }

        clipboard.operation = ClipboardOperation::Copy;
        Ok(())
    }

    pub fn copy_text(&self, text: &str) -> io::Result<()> {
        log_info!("Copying text to clipboard");
        let mut clipboard = self.0.lock().unwrap();

        // Set in system clipboard
        if let Some(system_clipboard) = clipboard.system_clipboard.as_mut() {
            if let Err(err) = system_clipboard.set_text(text.to_string()) {
                log_error!(format!("Failed to set text in system clipboard: {}", err).as_str());
            }
        }

        // Set in internal clipboard
        clipboard.content = ClipboardContent::TextContent(text.to_string());
        clipboard.operation = ClipboardOperation::Copy;
        log_info!("Copied text content to clipboard");

        Ok(())
    }

    pub fn cut_item(&self, path: &Path) -> io::Result<()> {
        log_info!(format!("Cutting item: {}", path.to_string_lossy()).as_str());
        
        if let Err(e) = self.copy_path(path) {
            log_error!(format!("Failed to copy path during cut operation: {}", e).as_str());
            return Err(e);
        }
        
        self.0.lock().unwrap().operation = ClipboardOperation::Cut;
        log_info!("Item marked for cut operation");
        Ok(())
    }

    pub fn paste_to_location(&self, target_dir: &Path) -> io::Result<()> {
        log_info!(format!("Pasting to location: {}", target_dir.to_string_lossy()).as_str());
        
        let mut clipboard = self.0.lock().unwrap();
        let content = clipboard.content.clone();
        let operation = clipboard.operation;

        match content {
            ClipboardContent::FilePath(source_path) => {
                log_info!(format!("Pasting file from: {}", source_path.to_string_lossy()).as_str());
                let file_name = match source_path.file_name() {
                    Some(name) => name,
                    None => {
                        let err = io::Error::new(io::ErrorKind::InvalidInput, "Invalid source path");
                        log_error!("Invalid source path for paste operation");
                        return Err(err);
                    }
                };
                
                let target_path = target_dir.join(file_name);
                
                if operation == ClipboardOperation::Cut {
                    log_info!(format!("Moving file to: {}", target_path.to_string_lossy()).as_str());
                    if let Err(e) = fs::rename(&source_path, &target_path) {
                        log_error!(format!("Failed to move file: {}", e).as_str());
                        return Err(e);
                    }
                } else {
                    log_info!(format!("Copying file to: {}", target_path.to_string_lossy()).as_str());
                    if let Err(e) = fs::copy(&source_path, &target_path) {
                        log_error!(format!("Failed to copy file: {}", e).as_str());
                        return Err(e);
                    }
                }
            },
            ClipboardContent::FolderPath(source_path) => {
                log_info!(format!("Pasting folder from: {}", source_path.to_string_lossy()).as_str());
                let folder_name = match source_path.file_name() {
                    Some(name) => name,
                    None => {
                        let err = io::Error::new(io::ErrorKind::InvalidInput, "Invalid source path");
                        log_error!("Invalid source path for paste operation");
                        return Err(err);
                    }
                };
                
                let target_path = target_dir.join(folder_name);
                
                if operation == ClipboardOperation::Cut {
                    log_info!(format!("Moving folder to: {}", target_path.to_string_lossy()).as_str());
                    if let Err(e) = fs::rename(&source_path, &target_path) {
                        log_error!(format!("Failed to move folder: {}", e).as_str());
                        return Err(e);
                    }
                } else {
                    log_info!(format!("Copying folder to: {}", target_path.to_string_lossy()).as_str());
                    if let Err(e) = copy_dir_recursive(&source_path, &target_path) {
                        log_error!(format!("Failed to copy folder recursively: {}", e).as_str());
                        return Err(e);
                    }
                }
            },
            ClipboardContent::TextContent(text) => {
                // Create a new text file with the content
                let file_path = target_dir.join("clipboard_content.txt");
                log_info!(format!("Pasting text content to file: {}", file_path.to_string_lossy()).as_str());
                if let Err(e) = fs::write(&file_path, text) {
                    log_error!(format!("Failed to write text content to file: {}", e).as_str());
                    return Err(e);
                }
            },
            ClipboardContent::BinaryContent(data) => {
                // Create a new binary file with the content
                let file_path = target_dir.join("clipboard_content.bin");
                log_info!(format!("Pasting binary content to file: {}", file_path.to_string_lossy()).as_str());
                if let Err(e) = fs::write(&file_path, data) {
                    log_error!(format!("Failed to write binary content to file: {}", e).as_str());
                    return Err(e);
                }
            },
            ClipboardContent::MultipleItems(paths) => {
                log_info!(format!("Pasting multiple items ({} items)", paths.len()).as_str());
                for path in paths {
                    if path.is_file() {
                        let file_name = match path.file_name() {
                            Some(name) => name,
                            None => {
                                log_error!(format!("Invalid source path: {}", path.to_string_lossy()).as_str());
                                continue;
                            }
                        };
                        
                        let target_path = target_dir.join(file_name);
                        
                        if operation == ClipboardOperation::Cut {
                            log_info!(format!("Moving file to: {}", target_path.to_string_lossy()).as_str());
                            if let Err(e) = fs::rename(&path, &target_path) {
                                log_error!(format!("Failed to move file {}: {}", path.to_string_lossy(), e).as_str());
                                return Err(e);
                            }
                        } else {
                            log_info!(format!("Copying file to: {}", target_path.to_string_lossy()).as_str());
                            if let Err(e) = fs::copy(&path, &target_path) {
                                log_error!(format!("Failed to copy file {}: {}", path.to_string_lossy(), e).as_str());
                                return Err(e);
                            }
                        }
                    } else if path.is_dir() {
                        let folder_name = match path.file_name() {
                            Some(name) => name,
                            None => {
                                log_error!(format!("Invalid source path: {}", path.to_string_lossy()).as_str());
                                continue;
                            }
                        };
                        
                        let target_path = target_dir.join(folder_name);
                        
                        if operation == ClipboardOperation::Cut {
                            log_info!(format!("Moving folder to: {}", target_path.to_string_lossy()).as_str());
                            if let Err(e) = fs::rename(&path, &target_path) {
                                log_error!(format!("Failed to move folder {}: {}", path.to_string_lossy(), e).as_str());
                                return Err(e);
                            }
                        } else {
                            log_info!(format!("Copying folder to: {}", target_path.to_string_lossy()).as_str());
                            if let Err(e) = copy_dir_recursive(&path, &target_path) {
                                log_error!(format!("Failed to copy folder {}: {}", path.to_string_lossy(), e).as_str());
                                return Err(e);
                            }
                        }
                    }
                }
            },
            ClipboardContent::None => {
                let err = io::Error::new(io::ErrorKind::Other, "Nothing to paste");
                log_error!("Nothing to paste: clipboard is empty");
                return Err(err);
            }
        }

        // Clear-cut operation after paste
        if operation == ClipboardOperation::Cut {
            clipboard.operation = ClipboardOperation::None;
            log_info!("Cut operation completed, clipboard operation reset");
        }
        
        log_info!("Paste operation completed successfully");
        Ok(())
    }

    pub fn has_content(&self) -> bool {
        !matches!(self.0.lock().unwrap().content, ClipboardContent::None)
    }
    
    pub fn get_operation(&self) -> ClipboardOperation {
        self.0.lock().unwrap().operation
    }
    
    pub fn copy_multiple_items(&self, paths: Vec<PathBuf>) -> io::Result<()> {
        if paths.is_empty() {
            log_warn!("Attempted to copy empty items list");
            return Ok(());
        }
        
        log_info!(format!("Copying multiple items ({} items)", paths.len()).as_str());
        let mut clipboard = self.0.lock().unwrap();
        clipboard.content = ClipboardContent::MultipleItems(paths);
        clipboard.operation = ClipboardOperation::Copy;
        Ok(())
    }
    
    pub fn cut_multiple_items(&self, paths: Vec<PathBuf>) -> io::Result<()> {
        if paths.is_empty() {
            log_warn!("Attempted to cut empty items list");
            return Ok(());
        }
        
        log_info!(format!("Cutting multiple items ({} items)", paths.len()).as_str());
        let mut clipboard = self.0.lock().unwrap();
        clipboard.content = ClipboardContent::MultipleItems(paths);
        clipboard.operation = ClipboardOperation::Cut;
        Ok(())
    }

    // For testing - initialize with custom clipboard contents
    #[cfg(test)]
    pub fn new_with_content(content: ClipboardContent, operation: ClipboardOperation) -> Self {
        let mut clipboard = Clipboard::new();
        clipboard.content = content;
        clipboard.operation = operation;
        Self(Arc::new(Mutex::new(clipboard)))
    }

    #[cfg(test)]
    pub fn get_content(&self) -> ClipboardContent {
        self.0.lock().unwrap().content.clone()
    }
}

// Helper function to recursively copy directories
fn copy_dir_recursive(source: &Path, destination: &Path) -> io::Result<()> {
    log_info!(format!("Recursively copying directory from {} to {}", 
              source.to_string_lossy(), 
              destination.to_string_lossy()).as_str());
    
    if !destination.exists() {
        if let Err(e) = fs::create_dir(destination) {
            log_error!(format!("Failed to create destination directory {}: {}", 
                      destination.to_string_lossy(), e).as_str());
            return Err(e);
        }
    }

    for entry_result in fs::read_dir(source)? {
        let entry = match entry_result {
            Ok(entry) => entry,
            Err(e) => {
                log_error!(format!("Failed to read directory entry: {}", e).as_str());
                return Err(e);
            }
        };
        
        let entry_path = entry.path();
        let file_name = entry.file_name();
        let destination_path = destination.join(file_name);

        if entry_path.is_dir() {
            if let Err(e) = copy_dir_recursive(&entry_path, &destination_path) {
                log_error!(format!("Failed to copy subdirectory {}: {}", 
                          entry_path.to_string_lossy(), e).as_str());
                return Err(e);
            }
        } else {
            if let Err(e) = fs::copy(&entry_path, &destination_path) {
                log_error!(format!("Failed to copy file {} to {}: {}", 
                          entry_path.to_string_lossy(), 
                          destination_path.to_string_lossy(), e).as_str());
                return Err(e);
            }
        }
    }
    
    log_info!("Directory copy completed successfully");
    Ok(())
}

#[cfg(test)]
mod tests_clipboard {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::{tempdir, TempDir};

    fn setup_test_dir() -> (TempDir, PathBuf) {
        let dir = tempdir().expect("Failed to create temp directory");
        let dir_path = dir.path().to_path_buf();
        (dir, dir_path)
    }

    fn create_test_file(dir: &Path, name: &str, content: &[u8]) -> PathBuf {
        let file_path = dir.join(name);
        let mut file = File::create(&file_path).expect("Failed to create test file");
        file.write_all(content).expect("Failed to write to test file");
        file_path
    }

    fn create_test_dir(dir: &Path, name: &str) -> PathBuf {
        let dir_path = dir.join(name);
        fs::create_dir(&dir_path).expect("Failed to create test directory");
        dir_path
    }

    #[test]
    fn test_clipboard_new() {
        let clipboard = ClipboardState::new();
        assert!(!clipboard.has_content());
        assert_eq!(clipboard.get_operation(), ClipboardOperation::None);
    }

    #[test]
    fn test_copy_file_path() {
        let clipboard = ClipboardState::new();
        let (dir, test_dir) = setup_test_dir();
        let test_file = create_test_file(&test_dir, "test.txt", b"test content");

        clipboard.copy_path(&test_file).expect("Failed to copy path");

        assert!(clipboard.has_content());
        assert_eq!(clipboard.get_operation(), ClipboardOperation::Copy);

        if let ClipboardContent::FilePath(path) = clipboard.get_content() {
            assert_eq!(path, test_file);
        } else {
            panic!("Expected FilePath content type");
        }

        // Keep the directory alive until the end of the test
        drop(dir);
    }

    #[test]
    fn test_copy_dir_path() {
        let clipboard = ClipboardState::new();
        let (dir, test_dir) = setup_test_dir();
        let test_subdir = create_test_dir(&test_dir, "test_subdir");

        clipboard.copy_path(&test_subdir).expect("Failed to copy directory path");

        assert!(clipboard.has_content());
        assert_eq!(clipboard.get_operation(), ClipboardOperation::Copy);

        if let ClipboardContent::FolderPath(path) = clipboard.get_content() {
            assert_eq!(path, test_subdir);
        } else {
            panic!("Expected FolderPath content type");
        }

        // Keep the directory alive until the end of the test
        drop(dir);
    }

    #[test]
    fn test_copy_file_content() {
        let clipboard = ClipboardState::new();
        let (dir, test_dir) = setup_test_dir();
        let test_file = create_test_file(&test_dir, "test.txt", b"test content");

        clipboard.copy_file_content(&test_file).expect("Failed to copy file content");

        assert!(clipboard.has_content());
        assert_eq!(clipboard.get_operation(), ClipboardOperation::Copy);

        if let ClipboardContent::TextContent(content) = clipboard.get_content() {
            assert_eq!(content, "test content");
        } else {
            panic!("Expected TextContent content type");
        }

        // Keep the directory alive until the end of the test
        drop(dir);
    }

    #[test]
    fn test_copy_binary_content() {
        let clipboard = ClipboardState::new();
        let (dir, test_dir) = setup_test_dir();
        // Create non-UTF8 binary data
        let binary_data = [0x80, 0xFF, 0x00, 0x7F];
        let test_file = create_test_file(&test_dir, "binary.bin", &binary_data);

        clipboard.copy_file_content(&test_file).expect("Failed to copy binary content");

        assert!(clipboard.has_content());
        assert_eq!(clipboard.get_operation(), ClipboardOperation::Copy);

        if let ClipboardContent::BinaryContent(content) = clipboard.get_content() {
            assert_eq!(content, binary_data);
        } else {
            panic!("Expected BinaryContent content type");
        }

        // Keep the directory alive until the end of the test
        drop(dir);
    }

    #[test]
    fn test_cut_item() {
        let clipboard = ClipboardState::new();
        let (dir, test_dir) = setup_test_dir();
        let test_file = create_test_file(&test_dir, "test.txt", b"test content");

        clipboard.cut_item(&test_file).expect("Failed to cut item");

        assert!(clipboard.has_content());
        assert_eq!(clipboard.get_operation(), ClipboardOperation::Cut);

        if let ClipboardContent::FilePath(path) = clipboard.get_content() {
            assert_eq!(path, test_file);
        } else {
            panic!("Expected FilePath content type");
        }

        // Keep the directory alive until the end of the test
        drop(dir);
    }

    #[test]
    fn test_paste_file() {
        let clipboard = ClipboardState::new();
        let (source_dir, source_path) = setup_test_dir();
        let (target_dir, target_path) = setup_test_dir();

        let test_file = create_test_file(&source_path, "test.txt", b"test content");
        clipboard.copy_path(&test_file).expect("Failed to copy path");

        clipboard.paste_to_location(&target_path).expect("Failed to paste");

        let pasted_file = target_path.join("test.txt");
        assert!(pasted_file.exists(), "Pasted file should exist");
        let content = fs::read_to_string(pasted_file).expect("Failed to read pasted file");
        assert_eq!(content, "test content");

        // Original file should still exist after copy
        assert!(test_file.exists(), "Original file should still exist after copy");

        // The clipboard operation remains Copy
        assert_eq!(clipboard.get_operation(), ClipboardOperation::Copy);

        drop(source_dir); // Keep the temporary directory alive until here
        drop(target_dir);
    }

    #[test]
    fn test_cut_and_paste_file() {
        let clipboard = ClipboardState::new();
        let (source_dir, source_path) = setup_test_dir();
        let (target_dir, target_path) = setup_test_dir();

        let test_file = create_test_file(&source_path, "test.txt", b"test content");
        clipboard.cut_item(&test_file).expect("Failed to cut item");

        clipboard.paste_to_location(&target_path).expect("Failed to paste");

        let pasted_file = target_path.join("test.txt");
        assert!(pasted_file.exists(), "Pasted file should exist");

        // Original file should NOT exist after cut and paste
        assert!(!test_file.exists(), "Original file should not exist after cut and paste");

        // The clipboard operation should reset after paste
        assert_eq!(clipboard.get_operation(), ClipboardOperation::None);

        drop(source_dir);
        drop(target_dir);
    }

    #[test]
    fn test_copy_and_paste_directory() {
        let clipboard = ClipboardState::new();
        let (source_dir, source_path) = setup_test_dir();
        let (target_dir, target_path) = setup_test_dir();

        let test_subdir = create_test_dir(&source_path, "test_subdir");
        let _test_file_in_subdir = create_test_file(&test_subdir, "file.txt", b"content in subdir");

        clipboard.copy_path(&test_subdir).expect("Failed to copy directory");
        clipboard.paste_to_location(&target_path).expect("Failed to paste directory");

        let pasted_dir = target_path.join("test_subdir");
        let pasted_file = pasted_dir.join("file.txt");

        assert!(pasted_dir.exists() && pasted_dir.is_dir(), "Pasted directory should exist");
        assert!(pasted_file.exists(), "File in pasted directory should exist");

        // Original directory should still exist after copy
        assert!(test_subdir.exists(), "Original directory should still exist after copy");

        drop(source_dir);
        drop(target_dir);
    }

    #[test]
    fn test_copy_multiple_items() {
        let clipboard = ClipboardState::new();
        let (dir, path) = setup_test_dir();

        let file1 = create_test_file(&path, "file1.txt", b"content 1");
        let file2 = create_test_file(&path, "file2.txt", b"content 2");
        let paths = vec![file1.clone(), file2.clone()];

        clipboard.copy_multiple_items(paths).expect("Failed to copy multiple items");

        assert!(clipboard.has_content());
        assert_eq!(clipboard.get_operation(), ClipboardOperation::Copy);

        if let ClipboardContent::MultipleItems(items) = clipboard.get_content() {
            assert_eq!(items.len(), 2);
            assert!(items.contains(&file1));
            assert!(items.contains(&file2));
        } else {
            panic!("Expected MultipleItems content type");
        }

        drop(dir);
    }

    #[test]
    fn test_cut_and_paste_multiple_items() {
        let clipboard = ClipboardState::new();
        let (source_dir, source_path) = setup_test_dir();
        let (target_dir, target_path) = setup_test_dir();

        let file1 = create_test_file(&source_path, "file1.txt", b"content 1");
        let file2 = create_test_file(&source_path, "file2.txt", b"content 2");
        let paths = vec![file1.clone(), file2.clone()];

        clipboard.cut_multiple_items(paths).expect("Failed to cut multiple items");
        clipboard.paste_to_location(&target_path).expect("Failed to paste multiple items");

        // Check files were moved
        assert!(!file1.exists(), "Original file1 should not exist after cut and paste");
        assert!(!file2.exists(), "Original file2 should not exist after cut and paste");

        let pasted_file1 = target_path.join("file1.txt");
        let pasted_file2 = target_path.join("file2.txt");

        assert!(pasted_file1.exists(), "Pasted file1 should exist");
        assert!(pasted_file2.exists(), "Pasted file2 should exist");

        // The clipboard operation should reset after paste
        assert_eq!(clipboard.get_operation(), ClipboardOperation::None);

        drop(source_dir);
        drop(target_dir);
    }

    #[test]
    fn test_empty_clipboard() {
        let clipboard = ClipboardState::new();
        let (dir, path) = setup_test_dir();

        let result = clipboard.paste_to_location(&path);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::Other);

        drop(dir);
    }

    #[test]
    fn test_custom_clipboard_creation() {
        let test_path = PathBuf::from("/test/path");
        let clipboard = ClipboardState::new_with_content(
            ClipboardContent::FilePath(test_path.clone()),
            ClipboardOperation::Copy
        );

        assert!(clipboard.has_content());
        assert_eq!(clipboard.get_operation(), ClipboardOperation::Copy);

        if let ClipboardContent::FilePath(path) = clipboard.get_content() {
            assert_eq!(path, test_path);
        } else {
            panic!("Expected FilePath content type");
        }
    }
}
