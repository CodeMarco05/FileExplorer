use crate::models;
use crate::models::{
    count_subfiles_and_subdirectories, format_system_time, get_access_permission_number,
    get_access_permission_string, get_directory_size_in_bytes, Entries,
};
use std::fs;
use std::fs::read_dir;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use zip::ZipWriter;
use zip::write::FileOptions;
use crate::state::clipboard_data::ClipboardState;
use crate::log_info;
use crate::log_error;

/// Opens a file at the given path and returns its contents as a string.
/// Should only be used for text files.
///
/// # Arguments
///
/// * `path` - A string slice that holds the path to the file to be opened.
///
/// # Returns
///
/// * `Ok(String)` - If the file was successfully opened and read.
/// * `Err(String)` - If there was an error during the opening or reading process.
///
/// # Example
///
/// ```rust
/// let result = open_file("/path/to/file.txt").await;
/// match result {
///     Ok(contents) => println!("File contents: {}", contents),
///     Err(err) => println!("Error opening file: {}", err),
/// }
/// ```
#[tauri::command]
pub async fn open_file(path: &str) -> Result<String, String> {
    let path_obj = Path::new(path);

    // Check if path exists
    if !path_obj.exists() {
        return Err(format!("File does not exist: {}", path));
    }

    // Check if path is a file
    if !path_obj.is_file() {
        return Err(format!("Path is not a file: {}", path));
    }

    // Read the file
    fs::read_to_string(path).map_err(|err| format!("Failed to read file: {}", err))
}

/// Opens a directory at the given path and returns its contents as a json string.
///
/// # Arguments
/// - `path` - A string slice that holds the path to the directory to be opened.
///
/// # Returns
/// - `Ok(Entries)` - If the directory was successfully opened and read.
/// - `Err(String)` - If there was an error during the opening or reading process.
///
/// # Example
/// ```rust
/// let result = open_directory("/path/to/directory").await;
/// match result {
///    Ok(entries) => {
///       for dir in entries.directories {
///          println!("Directory: {}", dir.name);
///       }
///      for file in entries.files {
///         println!("File: {}", file.name);
///      }
///   },
///   Err(err) => println!("Error opening directory: {}", err),
/// }
/// ```
#[tauri::command]
pub async fn open_directory(path: String) -> Result<String, String> {
    let path_obj = Path::new(&path);

    // Check if path exists
    if !path_obj.exists() {
        return Err(format!("Directory does not exist: {}", path));
    }

    // Check if path is a directory
    if !path_obj.is_dir() {
        return Err(format!("Path is not a directory: {}", path));
    }

    let mut directories = Vec::new();
    let mut files = Vec::new();

    for entry in read_dir(path_obj).map_err(|err| format!("Failed to read directory: {}", err))? {
        let entry = entry.map_err(|err| format!("Failed to read entry: {}", err))?;
        let file_type = entry
            .file_type()
            .map_err(|err| format!("Failed to get file type: {}", err))?;
        let path_of_entry = entry.path();
        let metadata = entry
            .metadata()
            .map_err(|err| format!("Failed to get metadata: {}", err))?;

        let (subfile_count, subdir_count) =
            count_subfiles_and_subdirectories(path_of_entry.to_str().unwrap());

        if file_type.is_dir() {
            directories.push(models::Directory {
                name: entry.file_name().to_str().unwrap().to_string(),
                path: path_of_entry.to_str().unwrap().to_string(),
                is_symlink: path_of_entry.is_symlink(),
                access_rights_as_string: get_access_permission_string(metadata.permissions(), true),
                access_rights_as_number: get_access_permission_number(metadata.permissions(), true),
                size_in_bytes: get_directory_size_in_bytes(path_of_entry.to_str().unwrap()),
                sub_file_count: subfile_count,
                sub_dir_count: subdir_count,
                created: format_system_time(metadata.created().unwrap()),
                last_modified: format_system_time(metadata.modified().unwrap()),
                accessed: format_system_time(metadata.accessed().unwrap()),
            });
        } else if file_type.is_file() {
            files.push(models::File {
                name: entry.file_name().to_str().unwrap().to_string(),
                path: path_of_entry.to_str().unwrap().to_string(),
                is_symlink: path_of_entry.is_symlink(),
                access_rights_as_string: get_access_permission_string(
                    metadata.permissions(),
                    false,
                ),
                access_rights_as_number: get_access_permission_number(metadata.permissions(), false),
                size_in_bytes: metadata.len(),
                created: format_system_time(metadata.created().unwrap()),
                last_modified: format_system_time(metadata.modified().unwrap()),
                accessed: format_system_time(metadata.accessed().unwrap()),
            });
        }
    }

    let entries = Entries { directories, files };

    // Convert the Entries struct to a JSON string
    let json = serde_json::to_string(&entries)
        .map_err(|err| format!("Failed to serialize entries: {}", err))?;
    Ok(json)
}

/// Creates a file at the given absolute path. Returns a string if there was an error.
/// This function does not create any parent directories.
///
/// # Arguments
/// - `file_path_abs` - A string slice that holds the absolute path to the file to be created.
///
/// # Returns
/// - `Ok(())` if the file was successfully created.
/// - `Err(String)` if there was an error during the creation process.
///
/// # Example
/// ```rust
/// let result = create_file("/path/to/file.txt").await;
/// match result {
///     Ok(_) => println!("File created successfully!"),
///     Err(err) => println!("Error creating file: {}", err),
/// }
/// ```
#[tauri::command]
pub async fn create_file(folder_path_abs: &str, file_name: &str) -> Result<(), String> {
    // Check if the folder path exists and is valid
    let path = Path::new(folder_path_abs);
    if !path.exists() {
        return Err(format!("Directory does not exist: {}", folder_path_abs));
    }
    if !path.is_dir() {
        return Err(format!("Path is no directory: {}", folder_path_abs));
    }

    // Concatenate the folder path and filename
    let file_path = path.join(file_name);

    // Create the file
    match fs::File::create(&file_path) {
        Ok(_) => Ok(()),
        Err(err) => Err(format!("File could not be created: {}", err)),
    }
}

/// Creates a directory at the given absolute path. Returns a string if there was an error.
/// This function does not create any parent directories.
/// 
/// # Arguments
/// - `folder_path_abs` - A string slice that holds the absolute path to the directory to be created.
/// 
/// # Returns
/// - `Ok(())` if the directory was successfully created.
/// - `Err(String)` if there was an error during the creation process.
/// 
/// # Example
/// ```rust
/// let result = create_directory("/path/to/directory", "new_folder").await;
/// match result {
///     Ok(_) => println!("Directory created successfully!"),
///     Err(err) => println!("Error creating directory: {}", err),
/// }
/// ```
#[tauri::command]
pub async fn create_directory(folder_path_abs: &str, folder_name: &str) -> Result<(), String> {
    // Check if the folder path exists and is valid
    let parent_path = Path::new(folder_path_abs);
    if !parent_path.exists() {
        return Err(format!("Parent directory does not exist: {}", folder_path_abs));
    }
    if !parent_path.is_dir() {
        return Err(format!("Path is not a directory: {}", folder_path_abs));
    }

    // Concatenate the parent path and new directory name
    let dir_path = parent_path.join(folder_name);

    // Create the directory
    match fs::create_dir(&dir_path) {
        Ok(_) => Ok(()),
        Err(err) => Err(format!("Failed to create directory: {}", err)),
    }
}

/// Renames a file or directory at the given path.
///
/// # Arguments
/// - `path` - The current path of the file or directory
/// - `new_path` - The new path for the file or directory
///
/// # Returns
/// - `Ok(())` if the rename operation was successful
/// - `Err(Error)` if there was an error during the operation
///
/// # Example
/// ```rust
/// let result = rename_file("/path/to/old_file.txt", "/path/to/new_file.txt").await;
/// match result {
///     Ok(_) => println!("File renamed successfully!"),
///     Err(err) => println!("Error renaming file: {}", err),
/// }
/// ```
#[tauri::command]
pub async fn rename(old_path: &str, new_path: &str) -> Result<(), String> {
    let old_path_obj = Path::new(old_path);
    let new_path_obj = Path::new(new_path);

    // Check if the old path exists
    if !old_path_obj.exists() {
        return Err(format!("File does not exist: {}", old_path));
    }

    // Check if the new path is valid
    if new_path_obj.exists() {
        return Err(format!("New path already exists: {}", new_path));
    }

    // Rename the file or directory
    fs::rename(old_path, new_path).map_err(|err| format!("Failed to rename: {}", err))
}

/// Deletes a file at the given path. Returns a string if there was an error.
/// This function moves the file to the trash instead of deleting it permanently.
///
/// # Arguments
/// - `path` - A string slice that holds the path to the file to be deleted.
///
/// # Returns
/// - `Ok(())` if the file was successfully deleted.
/// - `Err(String)` if there was an error during the deletion process.
///
/// # Example
/// ```rust
/// let result = delete_file("/path/to/file.txt").await;
/// match result {
///   Ok(_) => println!("File deleted successfully!"),
///   Err(err) => println!("Error deleting file: {}", err),
/// }
/// ```
#[tauri::command]
pub async fn move_to_trash(path: &str) -> Result<(), String> {
    match trash::delete(path) {
        Ok(_) => Ok(()),
        Err(err) => Err(format!("Failed to move file or directory to trash: {}", err)),
    }
}

/// Copies a file or directory from the source path to the destination path.
/// This function does not create any parent directories.
/// It will overwrite the destination if it already exists.
/// If the source is a directory, it will recursively copy all files and subdirectories.
/// 
/// # Arguments
/// - `source_path` - A string slice that holds the path to the source file or directory.
/// - `destination_path` - A string slice that holds the path to the destination.
/// 
/// # Returns
/// - `Ok(u64)` - The total size of copied files in bytes.
/// - `Err(String)` - If there was an error during the copy process.
/// 
/// # Example
/// ```rust
/// let result = copy_file_or_dir("/path/to/source.txt", "/path/to/destination.txt").await;
/// match result {
///     Ok(size) => println!("File copied successfully! Size: {} bytes", size),
///     Err(err) => println!("Error copying file: {}", err),
/// }
/// ```
#[tauri::command]
pub async fn copy_file_or_dir(source_path: &str, destination_path: &str) -> Result<u64, String> {
    // Check if the source path exists
    if !Path::new(source_path).exists() {
        return Err(format!("Source path does not exist: {}", source_path));
    }

    // Check if the destination path is valid
    if Path::new(destination_path).exists() {
        return Err(format!("Destination path already exists: {}", destination_path));
    }
    
    if Path::new(source_path).is_dir() {
        // If the source is a directory, recursively copy it
        let mut total_size = 0;
        
        // Create the destination directory
        fs::create_dir_all(destination_path)
            .map_err(|err| format!("Failed to create destination directory: {}", err))?;
        
        // Read all entries in the source directory
        for entry in fs::read_dir(source_path)
            .map_err(|err| format!("Failed to read source directory: {}", err))? {
            
            let entry = entry.map_err(|err| format!("Failed to read directory entry: {}", err))?;
            let entry_path = entry.path();
            let file_name = entry.file_name();
            let dest_path = Path::new(destination_path).join(file_name);
            
            if entry_path.is_file() {
                // Copy file
                let size = fs::copy(&entry_path, &dest_path)
                    .map_err(|err| format!("Failed to copy file '{}': {}", entry_path.display(), err))?;
                total_size += size;
            } else if entry_path.is_dir() {
                // Recursively copy subdirectory
                let sub_size = Box::pin(copy_file_or_dir(
                    entry_path.to_str().unwrap(),
                    dest_path.to_str().unwrap()
                )).await?;
                total_size += sub_size;
            }
        }
        
        Ok(total_size)
    } else {
        // Copy a single file
        let size = fs::copy(source_path, destination_path)
            .map_err(|err| format!("Failed to copy file: {}", err))?;
        Ok(size)
    }
}
/// Zips files and directories to a destination zip file.
/// If only one source path is provided and no destination is specified, creates a zip file with the same name.
/// For multiple source paths, the destination path must be specified.
///
/// # Arguments
/// * `source_paths` - Vector of paths to files/directories to be zipped
/// * `destination_path` - Optional destination path for the zip file
///
/// # Returns
/// * `Ok(())` - If the zip file was successfully created
/// * `Err(String)` - If there was an error during the zipping process
///
/// # Example
/// ```rust
/// // Single file/directory with auto destination
/// let result = zip(vec!["/path/to/file.txt"], None).await;
/// 
/// // Multiple files to specific destination
/// let result = zip(
///     vec!["/path/to/file1.txt", "/path/to/dir1"],
///     Some("/path/to/archive.zip")
/// ).await;
/// ```
#[tauri::command]
pub async fn zip(source_paths: Vec<String>, destination_path: Option<String>) -> Result<(), String> {
    if source_paths.is_empty() {
        return Err("No source paths provided".to_string());
    }

    // If single source and no destination, use source name with .zip
    let zip_path = if source_paths.len() == 1 && destination_path.is_none() {
        Path::new(&source_paths[0]).with_extension("zip")
    } else if let Some(dest) = destination_path {
        Path::new(&dest).to_path_buf()
    } else {
        return Err("Destination path required for multiple sources".to_string());
    };

    // Create zip file
    let zip_file = fs::File::create(&zip_path)
        .map_err(|e| format!("Failed to create zip file: {}", e))?;

    let mut zip = ZipWriter::new(zip_file);
    let options: FileOptions<()> = FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    // Process each source path
    for source_path in source_paths {
        let source = Path::new(&source_path);
        if !source.exists() {
            return Err(format!("Source path does not exist: {}", source_path));
        }

        let base_name = source.file_name()
            .ok_or_else(|| "Invalid source name".to_string())?
            .to_str()
            .ok_or_else(|| "Invalid characters in source name".to_string())?;

        if source.is_file() {
            zip.start_file(base_name, options)
                .map_err(|e| format!("Error adding file to zip: {}", e))?;
            let content = fs::read(source)
                .map_err(|e| format!("Error reading file: {}", e))?;
            zip.write_all(&content)
                .map_err(|e| format!("Error writing to zip: {}", e))?;
        } else if source.is_dir() {
            for entry in walkdir::WalkDir::new(source) {
                let entry = entry.map_err(|e| format!("Error reading directory: {}", e))?;
                let path = entry.path();

                if path.is_file() {
                    let relative = path.strip_prefix(source)
                        .map_err(|e| format!("Error creating relative path: {}", e))?;
                    let name = format!("{}/{}", base_name, relative.to_str()
                        .ok_or_else(|| "Invalid characters in path".to_string())?
                        .replace('\\', "/"));

                    zip.start_file(&name, options)
                        .map_err(|e| format!("Error adding file to zip: {}", e))?;
                    let content = fs::read(path)
                        .map_err(|e| format!("Error reading file: {}", e))?;
                    zip.write_all(&content)
                        .map_err(|e| format!("Error writing to zip: {}", e))?;
                }
            }
        }
    }

    zip.finish().map_err(|e| format!("Error finalizing zip file: {}", e))?;
    Ok(())
}

/// Extracts zip files to specified destinations.
/// If extracting a single zip file without a specified destination,
/// extracts to a directory with the same name as the zip file.
///
/// # Arguments
/// * `zip_paths` - Vector of paths to zip files
/// * `destination_path` - Optional destination directory for extraction
///
/// # Returns
/// * `Ok(())` - If all zip files were successfully extracted
/// * `Err(String)` - If there was an error during extraction
///
/// # Example
/// ```rust
/// // Single zip with auto destination
/// let result = unzip(vec!["/path/to/archive.zip"], None).await;
/// 
/// // Multiple zips to specific destination
/// let result = unzip(
///     vec!["/path/to/zip1.zip", "/path/to/zip2.zip"],
///     Some("/path/to/extracted")
/// ).await;
/// ```
#[tauri::command]
pub async fn unzip(zip_paths: Vec<String>, destination_path: Option<String>) -> Result<(), String> {
    if zip_paths.is_empty() {
        return Err("No zip files provided".to_string());
    }

    for zip_path in zip_paths.clone() {
        let zip_path = Path::new(&zip_path);
        if !zip_path.exists() {
            return Err(format!("Zip file does not exist: {}", zip_path.display()));
        }

        // Determine extraction path for this zip
        let extract_path = if zip_paths.len() == 1 && destination_path.is_none() {
            // For single zip without destination, use zip name without extension
            zip_path.with_extension("")
        } else if let Some(dest) = &destination_path {
            // For multiple zips or specified destination, create subdirectory for each zip
            let zip_name = zip_path.file_stem()
                .ok_or_else(|| "Invalid zip filename".to_string())?;
            Path::new(dest).join(zip_name)
        } else {
            return Err("Destination path required for multiple zip files".to_string());
        };

        // Create extraction directory
        fs::create_dir_all(&extract_path)
            .map_err(|e| format!("Failed to create extraction directory: {}", e))?;

        // Open and extract zip file
        let file = fs::File::open(zip_path)
            .map_err(|e| format!("Failed to open zip file: {}", e))?;
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| format!("Failed to read zip archive: {}", e))?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)
                .map_err(|e| format!("Failed to read zip entry: {}", e))?;
            let outpath = extract_path.join(file.mangled_name());

            if file.name().ends_with('/') {
                fs::create_dir_all(&outpath)
                    .map_err(|e| format!("Failed to create directory '{}': {}", outpath.display(), e))?;
            } else {
                if let Some(parent) = outpath.parent() {
                    if !parent.exists() {
                        fs::create_dir_all(parent)
                            .map_err(|e| format!("Failed to create parent directory '{}': {}", parent.display(), e))?;
                    }
                }
                let mut outfile = fs::File::create(&outpath)
                    .map_err(|e| format!("Failed to create file '{}': {}", outpath.display(), e))?;
                std::io::copy(&mut file, &mut outfile)
                    .map_err(|e| format!("Failed to write file '{}': {}", outpath.display(), e))?;
            }
        }
    }

    Ok(())
}

/// Pastes the current contents of the clipboard to the specified location.
/// If the clipboard contains a file or directory path, it will be copied or moved
/// (depending on whether it was copied or cut) to the destination.
/// If the clipboard contains text content, it will be saved as a text file.
///
/// # Arguments
/// * `clipboard_state` - The application's clipboard state
/// * `destination_path` - The destination path where the clipboard content will be pasted
///
/// # Returns
/// * `Ok(())` - If the paste operation was successful
/// * `Err(String)` - If there was an error during the paste operation
///
/// # Example
/// ```rust
/// let result = paste_from_clipboard(&clipboard_state, "/path/to/destination").await;
/// match result {
///     Ok(_) => println!("Content pasted successfully!"),
///     Err(err) => println!("Error pasting content: {}", err),
/// }
/// ```
#[tauri::command]
pub async fn paste_from_clipboard(
    clipboard_state: Arc<Mutex<ClipboardState>>,
    destination_path: &str
) -> Result<(), String> {
    log_info!(format!("Attempting to paste clipboard content to: {}", destination_path).as_str());

    let dest_path = Path::new(destination_path);

    // Check if destination path exists and is a directory
    if !dest_path.exists() {
        log_error!(format!("Destination path does not exist: {}", destination_path).as_str());
        return Err(format!("Destination path does not exist: {}", destination_path));
    }

    if !dest_path.is_dir() {
        log_error!(format!("Destination path is not a directory: {}", destination_path).as_str());
        return Err(format!("Destination path is not a directory: {}", destination_path));
    }

    let clipboard_state = clipboard_state.lock().unwrap();
    // Check if clipboard has content
    if !clipboard_state.has_content() {
        log_error!("Paste operation failed: clipboard is empty");
        return Err("Clipboard is empty".to_string());
    }

    log_info!("Clipboard has content, starting paste operation");

    // Perform the paste operation
    match clipboard_state.paste_to_location(dest_path) {
        Ok(_) => {
            log_info!("Successfully pasted clipboard content to destination");
            Ok(())
        },
        Err(err) => {
            log_error!(format!("Failed to paste clipboard content: {}", err).as_str());
            Err(format!("Failed to paste clipboard content: {}", err))
        }
    }
}

/// Copies a file or directory to the clipboard for later pasting.
/// This makes the path available in both the application's internal clipboard
/// and the system clipboard if possible.
///
/// # Arguments
/// * `clipboard_state` - The application's clipboard state
/// * `path` - Path to the file or directory to be copied
///
/// # Returns
/// * `Ok(())` - If the path was successfully copied to clipboard
/// * `Err(String)` - If there was an error during the copy operation
///
/// # Example
/// ```rust
/// let result = copy_to_clipboard(&clipboard_state, "/path/to/file.txt").await;
/// match result {
///     Ok(_) => println!("Path copied to clipboard successfully!"),
///     Err(err) => println!("Error copying to clipboard: {}", err),
/// }
/// ```
#[tauri::command]
pub async fn copy_to_clipboard(
    clipboard_state: Arc<Mutex<ClipboardState>>,
    path: &str
) -> Result<(), String> {
    log_info!(format!("Copying to clipboard: {}", path).as_str());

    let path_obj = Path::new(path);

    // Check if path exists
    if !path_obj.exists() {
        log_error!(format!("Path does not exist: {}", path).as_str());
        return Err(format!("Path does not exist: {}", path));
    }

    // Copy to clipboard
    let clipboard = clipboard_state.lock().unwrap();
    match clipboard.copy_path(path_obj) {
        Ok(_) => {
            log_info!("Successfully copied path to clipboard");
            Ok(())
        },
        Err(e) => {
            let error_msg = format!("Failed to copy to clipboard: {}", e);
            log_error!(error_msg.as_str());
            Err(error_msg)
        }
    }
}

#[tauri::command]
pub async fn cut(
    clipboard_state: Arc<Mutex<ClipboardState>>,
    path: &str
) -> Result<(), String> {
    log_info!(format!("Cutting to clipboard: {}", path).as_str());

    let path_obj = Path::new(path);

    // Check if path exists
    if !path_obj.exists() {
        log_error!(format!("Path does not exist: {}", path).as_str());
        return Err(format!("Path does not exist: {}", path));
    }

    // Cut to clipboard
    let clipboard = clipboard_state.lock().unwrap();

    match clipboard.cut_item(path_obj) {
        Ok(_) => {
            log_info!("Successfully cut path to clipboard");
            Ok(())
        },
        Err(e) => {
            let error_msg = format!("Failed to cut to clipboard: {}", e);
            log_error!(error_msg.as_str());
            Err(error_msg)
        }
    }
}

#[tauri::command]
pub async fn cut_multiple_items(
    clipboard_state: Arc<Mutex<ClipboardState>>,
    paths: Vec<String>
) -> Result<(), String> {
    log_info!(format!("Cutting multiple items to clipboard: {} items", paths.len()).as_str());

    // Convert paths to PathBuf
    let path_bufs: Vec<PathBuf> = paths.iter()
        .map(|path| PathBuf::from(path))
        .collect();

    // Verify all paths exist
    for path in &path_bufs {
        if !path.exists() {
            log_error!(format!("Path does not exist: {}", path.display()).as_str());
            return Err(format!("Path does not exist: {}", path.display()));
        }
    }

    // Cut to clipboard
    let clipboard = clipboard_state.lock().unwrap();

    match clipboard.cut_multiple_items(path_bufs) {
        Ok(_) => {
            log_info!("Successfully cut multiple items to clipboard");
            Ok(())
        },
        Err(e) => {
            let error_msg = format!("Failed to cut multiple items to clipboard: {}", e);
            log_error!(error_msg.as_str());
            Err(error_msg)
        }
    }
}

#[cfg(test)]
mod tests_file_system_operation_commands {
    use std::sync::{Arc, Mutex};
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn open_file_test() {
        use std::io::Write;
        use tempfile::tempdir;

        // Create a temporary directory (automatically deleted when out of scope)
        let temp_dir = tempdir().expect("Failed to create temporary directory");

        // Create a test file in the temporary directory
        let mut test_path = temp_dir.path().to_path_buf();
        test_path.push("open_file_test.txt");

        // Write some content to the test file
        let mut test_file = fs::File::create(&test_path).expect("Failed to create test file");
        writeln!(test_file, "Hello, world!").expect("Failed to write to test file");

        // Ensure the file exists
        assert!(test_path.exists(), "Test file should exist before reading");

        // Open the file and read its contents
        let result = open_file(test_path.to_str().unwrap()).await;

        // Verify that the operation was successful
        assert!(result.is_ok(), "Failed to open file: {:?}", result);

        // Verify the file contents
        assert_eq!(
            result.unwrap(),
            "Hello, world!\n",
            "File contents do not match expected value"
        );
    }

    #[tokio::test]
    async fn move_file_to_trash_test() {
        use tempfile::tempdir;

        // Create a temporary directory (automatically deleted when out of scope)
        let temp_dir = tempdir().expect("Failed to create temporary directory");

        // Create a test file in the temporary directory
        let mut test_path = temp_dir.path().to_path_buf();
        test_path.push("move_to_trash_test.txt");

        // Create the test file
        fs::File::create(&test_path).unwrap();

        // Ensure the file exists
        assert!(test_path.exists(), "Test file should exist before deletion");

        eprintln!("Test file exists: {:?}", test_path);

        // Move the file to the trash
        let result = move_to_trash(test_path.to_str().unwrap()).await;

        // Verify that the operation was successful
        assert!(result.is_ok(), "Failed to move file to trash: {:?}", result);

        // Verify that the file no longer exists at the original path
        assert!(
            !test_path.exists(),
            "File should no longer exist at the original path"
        );

        // No manual cleanup needed, as the temporary directory is automatically deleted
    }

    #[tokio::test]
    async fn create_file_test() {
        use tempfile::tempdir;

        // Create a temporary directory (automatically deleted when out of scope)
        let temp_dir = tempdir().expect("Failed to create temporary directory");

        // Create a test file path in the temporary directory
        let test_path = temp_dir.path().join("create_file_test.txt");

        // Call the function to create the file
        let result = create_file(temp_dir.path().to_str().unwrap(), "create_file_test.txt").await;

        // Verify that the operation was successful
        assert!(result.is_ok(), "Failed to create file: {:?}", result);

        // Verify that the file exists at the specified pat´ßp0
        assert!(test_path.exists(), "File should exist after creation");
    }
    
    #[tokio::test]
    async fn create_directory_test() {
        use tempfile::tempdir;

        // Create a temporary directory (automatically deleted when out of scope)
        let temp_dir = tempdir().expect("Failed to create temporary directory");

        // Create a test directory path in the temporary directory
        let test_path = temp_dir.path().join("create_directory_test");

        // Call the function to create the directory
        let result = create_directory(temp_dir.path().to_str().unwrap(), "create_directory_test").await;

        // Verify that the operation was successful
        assert!(result.is_ok(), "Failed to create directory: {:?}", result);

        // Verify that the directory exists at the specified path
        assert!(test_path.exists(), "Directory should exist after creation");
    }

    #[tokio::test]
    async fn open_directory_test() {
        use std::io::Write;
        use tempfile::tempdir;

        // Create a temporary directory (automatically deleted when out of scope)
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        println!("Temporary directory created: {:?}", temp_dir.path());

        // Create a subdirectory
        let sub_dir_path = temp_dir.path().join("subdir");
        fs::create_dir(&sub_dir_path).expect("Failed to create subdirectory");
        println!("Temporary subdirectory created: {:?}", sub_dir_path);

        // Create files in the root directory
        let file1_path = temp_dir.path().join("file1.txt");
        let mut file1 = fs::File::create(&file1_path).expect("Failed to create file1");
        writeln!(file1, "File 1 content").expect("Failed to write to file1");
        println!("File 1 created: {:?}", file1_path);

        let file2_path = temp_dir.path().join("file2.txt");
        let mut file2 = fs::File::create(&file2_path).expect("Failed to create file2");
        writeln!(file2, "File 2 content").expect("Failed to write to file2");
        println!("File 2 created: {:?}", file2_path);

        // Create files in the subdirectory
        let sub_file1_path = sub_dir_path.join("sub_file1.txt");
        let mut sub_file1 = fs::File::create(&sub_file1_path).expect("Failed to create sub_file1");
        writeln!(sub_file1, "Sub File 1 content").expect("Failed to write to sub_file1");
        println!("Sub File 1 created: {:?}", sub_file1_path);

        let sub_file2_path = sub_dir_path.join("sub_file2.txt");
        let mut sub_file2 = fs::File::create(&sub_file2_path).expect("Failed to create sub_file2");
        writeln!(sub_file2, "Sub File 2 content").expect("Failed to write to sub_file2");
        println!("Sub File 2 created: {:?}", sub_file2_path);

        // Call the open_directory function
        let result = open_directory(temp_dir.path().to_str().unwrap().to_string()).await;

        // Verify that the operation was successful
        assert!(result.is_ok(), "Failed to open directory: {:?}", result);

        let entries = result.unwrap();
        let entries: Entries = serde_json::from_str(&entries).expect("Failed to parse JSON");

        // Verify directories
        assert_eq!(entries.directories.len(), 1, "Expected 1 subdirectory");
        assert_eq!(
            entries.directories[0].name, "subdir",
            "Subdirectory name does not match"
        );

        // Verify files in the root directory
        assert_eq!(
            entries.files.len(),
            2,
            "Expected 2 files in the root directory"
        );
        let file_names: Vec<String> = entries.files.iter().map(|f| f.name.clone()).collect();
        assert!(
            file_names.contains(&"file1.txt".to_string()),
            "file1.txt not found"
        );
        assert!(
            file_names.contains(&"file2.txt".to_string()),
            "file2.txt not found"
        );

        // Verify subdirectory contents
        let subdir_result = open_directory(sub_dir_path.to_str().unwrap().to_string()).await;
        assert!(
            subdir_result.is_ok(),
            "Failed to open subdirectory: {:?}",
            subdir_result
        );

        let subdir_entries = subdir_result.unwrap();
        let subdir_entries: Entries =
            serde_json::from_str(&subdir_entries).expect("Failed to parse JSON");
        assert_eq!(
            subdir_entries.files.len(),
            2,
            "Expected 2 files in the subdirectory"
        );
        let sub_file_names: Vec<String> = subdir_entries
            .files
            .iter()
            .map(|f| f.name.clone())
            .collect();
        assert!(
            sub_file_names.contains(&"sub_file1.txt".to_string()),
            "sub_file1.txt not found"
        );
        assert!(
            sub_file_names.contains(&"sub_file2.txt".to_string()),
            "sub_file2.txt not found"
        );
    }
    
    #[tokio::test]
    async fn rename_file_test() {
        use tempfile::tempdir;

        // Create a temporary directory (automatically deleted when out of scope)
        let temp_dir = tempdir().expect("Failed to create temporary directory");

        // Create a test file in the temporary directory
        let mut test_path = temp_dir.path().to_path_buf();
        test_path.push("rename_file_test.txt");

        // Create the test file
        fs::File::create(&test_path).unwrap();

        // Ensure the file exists
        assert!(test_path.exists(), "Test file should exist before renaming");

        // Rename the file
        let new_name = "renamed_file.txt";
        let new_path = temp_dir.path().join(new_name);
        let result = rename(test_path.to_str().unwrap(), new_path.to_str().unwrap()).await;

        // Verify that the operation was successful
        assert!(result.is_ok(), "Failed to rename file: {:?}", result);

        // Verify that the file exists at the new path
        assert!(new_path.exists(), "File should exist at the new path");
    }
    
    #[tokio::test]
    async fn rename_directory_test(){
        use tempfile::tempdir;

        // Create a temporary directory (automatically deleted when out of scope)
        let temp_dir = tempdir().expect("Failed to create temporary directory");

        // Create a test directory in the temporary directory
        let mut test_path = temp_dir.path().to_path_buf();
        test_path.push("rename_directory_test");

        // Create the test directory
        fs::create_dir(&test_path).unwrap();

        // Ensure the directory exists
        assert!(test_path.exists(), "Test directory should exist before renaming");

        // Rename the directory
        let new_name = "renamed_directory";
        let new_path = temp_dir.path().join(new_name);
        let result = rename(test_path.to_str().unwrap(), new_path.to_str().unwrap()).await;

        // Verify that the operation was successful
        assert!(result.is_ok(), "Failed to rename directory: {:?}", result);

        // Verify that the directory exists at the new path
        assert!(new_path.exists(), "Directory should exist at the new path");
    }
    
    #[tokio::test]
    async fn copy_file_test() {
        use tempfile::tempdir;

        // Create a temporary directory (automatically deleted when out of scope)
        let temp_dir = tempdir().expect("Failed to create temporary directory");

        // Create a test file in the temporary directory
        let mut test_path = temp_dir.path().to_path_buf();
        test_path.push("copy_file_test.txt");

        // Create the test file
        fs::File::create(&test_path).unwrap();

        // Ensure the file exists
        assert!(test_path.exists(), "Test file should exist before copying");

        // Copy the file
        let new_name = "copied_file.txt";
        let new_path = temp_dir.path().join(new_name);
        let result = copy_file_or_dir(test_path.to_str().unwrap(), new_path.to_str().unwrap()).await;

        // Verify that the operation was successful
        assert!(result.is_ok(), "Failed to copy file: {:?}", result);

        // Verify that the copied file exists at the new path
        assert!(new_path.exists(), "Copied file should exist at the new path");
        
        // Verify the old file still exists
        assert!(test_path.exists(), "Original file should still exist");
    }
    
    #[tokio::test]
    async fn copy_directory_test() {
        use std::io::Write;
        use tempfile::tempdir;

        // Create a temporary directory (automatically deleted when out of scope)
        let temp_dir = tempdir().expect("Failed to create temporary directory");

        // Create a test directory in the temporary directory
        let test_path = temp_dir.path().join("copy_directory_test");
        fs::create_dir(&test_path).unwrap();

        // Create a file in the test directory
        let file_in_dir_path = test_path.join("file_in_dir.txt");
        let mut file_in_dir = fs::File::create(&file_in_dir_path).expect("Failed to create file in directory");
        writeln!(file_in_dir, "Content of file in directory").expect("Failed to write to file");

        // Create a subdirectory
        let subdir_path = test_path.join("subdir");
        fs::create_dir(&subdir_path).unwrap();

        // Create a file in the subdirectory
        let file_in_subdir_path = subdir_path.join("file_in_subdir.txt");
        let mut file_in_subdir = fs::File::create(&file_in_subdir_path).expect("Failed to create file in subdirectory");
        writeln!(file_in_subdir, "Content of file in subdirectory").expect("Failed to write to file");

        // Ensure the directory structure exists
        assert!(test_path.exists(), "Test directory should exist before copying");
        assert!(file_in_dir_path.exists(), "File in directory should exist before copying");
        assert!(subdir_path.exists(), "Subdirectory should exist before copying");
        assert!(file_in_subdir_path.exists(), "File in subdirectory should exist before copying");

        // Copy the directory
        let copied_dir_name = "copied_directory";
        let copied_dir_path = temp_dir.path().join(copied_dir_name);
        let result = copy_file_or_dir(test_path.to_str().unwrap(), copied_dir_path.to_str().unwrap()).await;

        // Verify that the operation was successful
        assert!(result.is_ok(), "Failed to copy directory: {:?}", result);

        // Verify that the copied directory exists
        assert!(copied_dir_path.exists(), "Copied directory should exist");

        // Verify that the file in the copied directory exists
        let copied_file_in_dir_path = copied_dir_path.join("file_in_dir.txt");
        assert!(copied_file_in_dir_path.exists(), "Copied file in directory should exist");

        // Verify that the subdirectory in the copied directory exists
        let copied_subdir_path = copied_dir_path.join("subdir");
        assert!(copied_subdir_path.exists(), "Copied subdirectory should exist");

        // Verify that the file in the copied subdirectory exists
        let copied_file_in_subdir_path = copied_subdir_path.join("file_in_subdir.txt");
        assert!(copied_file_in_subdir_path.exists(), "Copied file in subdirectory should exist");
        
        // Verify the original directory structure still exists
        assert!(test_path.exists(), "Original directory should still exist");
        assert!(file_in_dir_path.exists(), "Original file in directory should still exist");
        assert!(subdir_path.exists(), "Original subdirectory should still exist");
        assert!(file_in_subdir_path.exists(), "Original file in subdirectory should still exist");
    }

    #[tokio::test]
    async fn zip_single_file_test() {
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        let test_file_path = temp_dir.path().join("test_file.txt");

        // Create and write to test file
        fs::write(&test_file_path, "Test content").expect("Failed to write test file");
        assert!(test_file_path.exists(), "Test file should exist before zipping");

        // Zip the file
        let result = zip(vec![test_file_path.to_str().unwrap().to_string()], None).await;
        assert!(result.is_ok(), "Failed to zip file: {:?}", result);

        // Check if zip file was created
        let zip_path = test_file_path.with_extension("zip");
        assert!(zip_path.exists(), "Zip file should exist after operation");

        // Verify zip contents
        let zip_file = fs::File::open(&zip_path).expect("Failed to open zip file");
        let mut archive = zip::ZipArchive::new(zip_file).expect("Failed to read zip archive");
        assert_eq!(archive.len(), 1, "Zip should contain exactly one file");
        
        let file = archive.by_index(0).expect("Failed to read file from zip");
        assert_eq!(file.name(), "test_file.txt", "Incorrect filename in zip");
    }

    #[tokio::test]
    async fn zip_multiple_files_test() {
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        
        // Create test files
        let file1_path = temp_dir.path().join("file1.txt");
        let file2_path = temp_dir.path().join("file2.txt");
        fs::write(&file1_path, "Content 1").expect("Failed to write file1");
        fs::write(&file2_path, "Content 2").expect("Failed to write file2");

        // Create destination zip path
        let zip_path = temp_dir.path().join("multiple_files.zip");

        // Zip multiple files
        let result = zip(
            vec![
                file1_path.to_str().unwrap().to_string(),
                file2_path.to_str().unwrap().to_string()
            ],
            Some(zip_path.to_str().unwrap().to_string())
        ).await;

        assert!(result.is_ok(), "Failed to zip multiple files: {:?}", result);
        assert!(zip_path.exists(), "Zip file should exist after operation");

        // Verify zip contents
        let zip_file = fs::File::open(&zip_path).expect("Failed to open zip file");
        let mut archive = zip::ZipArchive::new(zip_file).expect("Failed to read zip archive");
        assert_eq!(archive.len(), 2, "Zip should contain exactly two files");
        
        let mut file_names: Vec<String> = (0..archive.len())
            .map(|i| archive.by_index(i).unwrap().name().to_string())
            .collect();
        file_names.sort();

        assert!(file_names.contains(&"file1.txt".to_string()), "file1.txt missing from zip");
        assert!(file_names.contains(&"file2.txt".to_string()), "file2.txt missing from zip");
    }

    #[tokio::test]
    async fn unzip_single_file_test() {
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        
        // Create a test zip file
        let zip_path = temp_dir.path().join("test.zip");
        let mut zip = zip::ZipWriter::new(fs::File::create(&zip_path).unwrap());
        
        zip.start_file::<_, ()>("test.txt", FileOptions::default()).unwrap();
        zip.write_all(b"Hello, World!").unwrap();
        zip.finish().unwrap();

        // Test extraction without specifying destination
        let result = unzip(
            vec![zip_path.to_str().unwrap().to_string()],
            None
        ).await;

        assert!(result.is_ok(), "Failed to extract zip: {:?}", result);
        
        // Verify extracted contents
        let extract_path = zip_path.with_extension("");
        let test_file = extract_path.join("test.txt");
        
        assert!(test_file.exists(), "Extracted test.txt should exist");
        assert_eq!(
            fs::read_to_string(test_file).unwrap(),
            "Hello, World!",
            "Extracted content should match"
        );
    }

    #[tokio::test]
    async fn unzip_multiple_files_test() {
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        
        // Create test zip files
        let zip1_path = temp_dir.path().join("test1.zip");
        let zip2_path = temp_dir.path().join("test2.zip");

        // Create content for first zip
        let mut zip1 = zip::ZipWriter::new(fs::File::create(&zip1_path).unwrap());
        zip1.start_file::<_, ()>("file1.txt", FileOptions::default()).unwrap();
        zip1.write_all(b"Content 1").unwrap();
        zip1.finish().unwrap();

        // Create content for second zip
        let mut zip2 = zip::ZipWriter::new(fs::File::create(&zip2_path).unwrap());
        zip2.start_file::<_, ()>("file2.txt", FileOptions::default()).unwrap();
        zip2.write_all(b"Content 2").unwrap();
        zip2.finish().unwrap();

        // Create extraction directory
        let extract_path = temp_dir.path().join("extracted_multiple");

        // Test multiple extraction
        let result = unzip(
            vec![
                zip1_path.to_str().unwrap().to_string(),
                zip2_path.to_str().unwrap().to_string()
            ],
            Some(extract_path.to_str().unwrap().to_string())
        ).await;

        assert!(result.is_ok(), "Failed to extract multiple zips: {:?}", result);
        
        // Verify extracted contents
        let file1 = extract_path.join("test1").join("file1.txt");
        let file2 = extract_path.join("test2").join("file2.txt");
        
        assert!(file1.exists(), "Extracted file1.txt should exist");
        assert!(file2.exists(), "Extracted file2.txt should exist");
        
        assert_eq!(
            fs::read_to_string(file1).unwrap(),
            "Content 1",
            "Extracted content 1 should match"
        );
        assert_eq!(
            fs::read_to_string(file2).unwrap(),
            "Content 2",
            "Extracted content 2 should match"
        );
    }

    #[tokio::test]
    async fn paste_from_clipboard_file_test() {
        use crate::state::clipboard_data::{ClipboardState, ClipboardContent, ClipboardOperation};

        // Create temporary source and destination directories
        let source_dir = tempdir().expect("Failed to create source temporary directory");
        let dest_dir = tempdir().expect("Failed to create destination temporary directory");

        // Create a test file in the source directory
        let test_file_path = source_dir.path().join("test_file.txt");
        fs::write(&test_file_path, "Test content").expect("Failed to create test file");
        assert!(test_file_path.exists(), "Test file should exist before copying");

        // Set up clipboard state with the file path
        let clipboard = ClipboardState::new_with_content(
            ClipboardContent::FilePath(test_file_path.clone()),
            ClipboardOperation::Copy
        );

        // Paste from clipboard to destination
        let result = paste_from_clipboard(
            Arc::new(Mutex::new(clipboard)),
            dest_dir.path().to_str().unwrap()
        ).await;

        // Verify operation was successful
        assert!(result.is_ok(), "Paste operation failed: {:?}", result);

        // Check if file was copied to destination
        let dest_file_path = dest_dir.path().join("test_file.txt");
        assert!(dest_file_path.exists(), "File should exist at destination after paste");
        assert!(test_file_path.exists(), "Source file should still exist after copy operation");

        // Verify content was copied correctly
        let content = fs::read_to_string(&dest_file_path).expect("Failed to read pasted file");
        assert_eq!(content, "Test content", "File content should match");
    }

    #[tokio::test]
    async fn paste_from_clipboard_cut_file_test() {
        use crate::state::clipboard_data::{ClipboardState, ClipboardContent, ClipboardOperation};

        // Create temporary source and destination directories
        let source_dir = tempdir().expect("Failed to create source temporary directory");
        let dest_dir = tempdir().expect("Failed to create destination temporary directory");

        // Create a test file in the source directory
        let test_file_path = source_dir.path().join("cut_test_file.txt");
        fs::write(&test_file_path, "Cut file content").expect("Failed to create test file");
        assert!(test_file_path.exists(), "Test file should exist before cutting");

        // Set up clipboard state with the file path and cut operation
        let clipboard = ClipboardState::new_with_content(
            ClipboardContent::FilePath(test_file_path.clone()),
            ClipboardOperation::Cut
        );

        // Paste from clipboard to destination
        let result = paste_from_clipboard(
            Arc::new(Mutex::new(clipboard)),
            dest_dir.path().to_str().unwrap()
        ).await;

        // Verify operation was successful
        assert!(result.is_ok(), "Paste operation failed: {:?}", result);

        // Check if file was moved to destination
        let dest_file_path = dest_dir.path().join("cut_test_file.txt");
        assert!(dest_file_path.exists(), "File should exist at destination after paste");
        assert!(!test_file_path.exists(), "Source file should no longer exist after cut operation");

        // Verify content was moved correctly
        let content = fs::read_to_string(&dest_file_path).expect("Failed to read pasted file");
        assert_eq!(content, "Cut file content", "File content should match");
    }

    #[tokio::test]
    async fn paste_from_clipboard_directory_test() {
        use crate::state::clipboard_data::{ClipboardState, ClipboardContent, ClipboardOperation};

        // Create temporary source and destination directories
        let source_dir = tempdir().expect("Failed to create source temporary directory");
        let dest_dir = tempdir().expect("Failed to create destination temporary directory");

        // Create a test directory in the source directory
        let test_dir_path = source_dir.path().join("test_dir");
        fs::create_dir(&test_dir_path).expect("Failed to create test directory");

        // Create a file in the test directory
        let file_in_dir_path = test_dir_path.join("file_in_dir.txt");
        fs::write(&file_in_dir_path, "Directory test content").expect("Failed to create file in directory");

        // Set up clipboard state with the directory path
        let clipboard = ClipboardState::new_with_content(
            ClipboardContent::FolderPath(test_dir_path.clone()),
            ClipboardOperation::Copy
        );

        // Paste from clipboard to destination
        let result = paste_from_clipboard(
            Arc::new(Mutex::new(clipboard)),
            dest_dir.path().to_str().unwrap()
        ).await;

        // Verify operation was successful
        assert!(result.is_ok(), "Paste operation failed: {:?}", result);

        // Check if directory was copied to destination
        let dest_dir_path = dest_dir.path().join("test_dir");
        let dest_file_path = dest_dir_path.join("file_in_dir.txt");

        assert!(dest_dir_path.exists() && dest_dir_path.is_dir(), "Directory should exist at destination after paste");
        assert!(dest_file_path.exists(), "File in directory should exist at destination after paste");

        // Verify content was copied correctly
        let content = fs::read_to_string(&dest_file_path).expect("Failed to read pasted file");
        assert_eq!(content, "Directory test content", "File content should match");
    }

    #[tokio::test]
    async fn paste_from_clipboard_text_test() {
        use crate::state::clipboard_data::{ClipboardState, ClipboardContent, ClipboardOperation};

        // Create temporary destination directory
        let dest_dir = tempdir().expect("Failed to create destination temporary directory");

        // Set up clipboard state with text content
        let clipboard = ClipboardState::new_with_content(
            ClipboardContent::TextContent("Clipboard text content".to_string()),
            ClipboardOperation::Copy
        );

        // Paste from clipboard to destination
        let result = paste_from_clipboard(
            Arc::new(Mutex::new(clipboard)),
            dest_dir.path().to_str().unwrap()
        ).await;

        // Verify operation was successful
        assert!(result.is_ok(), "Paste operation failed: {:?}", result);

        // Check if text file was created in destination
        let dest_file_path = dest_dir.path().join("clipboard_content.txt");
        assert!(dest_file_path.exists(), "Text file should exist at destination after paste");

        // Verify content was saved correctly
        let content = fs::read_to_string(&dest_file_path).expect("Failed to read pasted file");
        assert_eq!(content, "Clipboard text content", "Text content should match");
    }

    #[tokio::test]
    async fn paste_from_clipboard_multiple_items_test() {
        use crate::state::clipboard_data::{ClipboardState, ClipboardContent, ClipboardOperation};

        // Create temporary source and destination directories
        let source_dir = tempdir().expect("Failed to create source temporary directory");
        let dest_dir = tempdir().expect("Failed to create destination temporary directory");

        // Create test files in the source directory
        let file1_path = source_dir.path().join("file1.txt");
        let file2_path = source_dir.path().join("file2.txt");
        fs::write(&file1_path, "File 1 content").expect("Failed to create file1");
        fs::write(&file2_path, "File 2 content").expect("Failed to create file2");

        // Set up clipboard state with multiple file paths
        let paths = vec![file1_path.clone(), file2_path.clone()];
        let clipboard = ClipboardState::new_with_content(
            ClipboardContent::MultipleItems(paths),
            ClipboardOperation::Copy
        );

        // Paste from clipboard to destination
        let result = paste_from_clipboard(
            Arc::new(Mutex::new(clipboard)),
            dest_dir.path().to_str().unwrap()
        ).await;

        // Verify operation was successful
        assert!(result.is_ok(), "Paste operation failed: {:?}", result);

        // Check if files were copied to destination
        let dest_file1_path = dest_dir.path().join("file1.txt");
        let dest_file2_path = dest_dir.path().join("file2.txt");

        assert!(dest_file1_path.exists(), "File1 should exist at destination after paste");
        assert!(dest_file2_path.exists(), "File2 should exist at destination after paste");

        // Verify content was copied correctly
        let content1 = fs::read_to_string(&dest_file1_path).expect("Failed to read pasted file1");
        let content2 = fs::read_to_string(&dest_file2_path).expect("Failed to read pasted file2");
        assert_eq!(content1, "File 1 content", "File1 content should match");
        assert_eq!(content2, "File 2 content", "File2 content should match");
    }

    #[tokio::test]
    async fn paste_from_clipboard_empty_clipboard_test() {
        use crate::state::clipboard_data::ClipboardState;

        // Create temporary destination directory
        let dest_dir = tempdir().expect("Failed to create destination temporary directory");

        // Create empty clipboard state (default constructor creates empty clipboard)
        let clipboard = ClipboardState::new();

        // Try to paste from empty clipboard
        let result = paste_from_clipboard(
            Arc::new(Mutex::new(clipboard)),
            dest_dir.path().to_str().unwrap()
        ).await;

        // Verify operation fails with appropriate error
        assert!(result.is_err(), "Paste operation should fail with empty clipboard");
        assert_eq!(result.unwrap_err(), "Clipboard is empty");
    }

    #[tokio::test]
    async fn paste_from_clipboard_invalid_destination_test() {
        use crate::state::clipboard_data::{ClipboardState, ClipboardContent, ClipboardOperation};

        // Create temporary source directory
        let source_dir = tempdir().expect("Failed to create source temporary directory");

        // Create a test file in the source directory
        let test_file_path = source_dir.path().join("test_file.txt");
        fs::write(&test_file_path, "Test content").expect("Failed to create test file");

        // Set up clipboard state with the file path
        let clipboard = ClipboardState::new_with_content(
            ClipboardContent::FilePath(test_file_path.clone()),
            ClipboardOperation::Copy
        );

        // Try to paste to non-existent destination
        let invalid_path = "/path/that/does/not/exist";
        let result = paste_from_clipboard(
            Arc::new(Mutex::new(clipboard)),
            invalid_path
        ).await;

        // Verify operation fails with appropriate error
        assert!(result.is_err(), "Paste operation should fail with invalid destination");
        assert!(result.unwrap_err().contains("Destination path does not exist"));
    }

    #[tokio::test]
    async fn copy_to_clipboard_file_test() {
        use crate::state::clipboard_data::{ClipboardState, ClipboardContent, ClipboardOperation};
        use std::io::Write;

        // Create a temporary directory
        let temp_dir = tempdir().expect("Failed to create temporary directory");

        // Create a test file
        let file_path = temp_dir.path().join("test_copy_file.txt");
        let mut test_file = fs::File::create(&file_path).expect("Failed to create test file");
        writeln!(test_file, "Test content for clipboard").expect("Failed to write to test file");

        // Create clipboard state
        let clipboard_state = Arc::new(Mutex::new(ClipboardState::new()));

        // Execute copy_to_clipboard command
        let result = copy_to_clipboard(clipboard_state.clone(), file_path.to_str().unwrap()).await;

        // Verify the operation was successful
        assert!(result.is_ok(), "Failed to copy to clipboard: {:?}", result);

        // Verify the clipboard content was set correctly
        let clipboard = clipboard_state.lock().unwrap();
        let clipboard_content = clipboard.get_content();

        match clipboard_content {
            ClipboardContent::FilePath(path) => {
                assert_eq!(path, file_path, "Path in clipboard doesn't match the copied path");
            },
            _ => panic!("Wrong clipboard content type. Expected FilePath."),
        }

        // Verify clipboard operation is set to Copy
        assert_eq!(clipboard.get_operation(), ClipboardOperation::Copy);
    }

    #[tokio::test]
    async fn copy_to_clipboard_directory_test() {
        use crate::state::clipboard_data::{ClipboardState, ClipboardContent, ClipboardOperation};

        // Create a temporary directory
        let temp_dir = tempdir().expect("Failed to create temporary directory");

        // Create a test subdirectory
        let dir_path = temp_dir.path().join("test_copy_dir");
        fs::create_dir(&dir_path).expect("Failed to create test directory");

        // Create clipboard state
        let clipboard_state = Arc::new(Mutex::new(ClipboardState::new()));

        // Execute copy_to_clipboard command
        let result = copy_to_clipboard(clipboard_state.clone(), dir_path.to_str().unwrap()).await;

        // Verify the operation was successful
        assert!(result.is_ok(), "Failed to copy directory to clipboard: {:?}", result);

        // Verify the clipboard content was set correctly
        let clipboard = clipboard_state.lock().unwrap();
        let clipboard_content = clipboard.get_content();

        match clipboard_content {
            ClipboardContent::FolderPath(path) => {
                assert_eq!(path, dir_path, "Path in clipboard doesn't match the copied directory path");
            },
            _ => panic!("Wrong clipboard content type. Expected FolderPath."),
        }

        // Verify clipboard operation is set to Copy
        assert_eq!(clipboard.get_operation(), ClipboardOperation::Copy);
    }

    #[tokio::test]
    async fn copy_to_clipboard_nonexistent_path_test() {
        use crate::state::clipboard_data::ClipboardState;

        // Create a non-existent path
        let non_existent_path = "/path/that/does/not/exist/file.txt";

        // Create clipboard state
        let clipboard_state = Arc::new(Mutex::new(ClipboardState::new()));

        // Execute copy_to_clipboard command with non-existent path
        let result = copy_to_clipboard(clipboard_state, non_existent_path).await;

        // Verify the operation fails with appropriate error
        assert!(result.is_err(), "Expected an error when copying a non-existent path");
        assert!(result.unwrap_err().contains("Path does not exist"));
    }

    #[tokio::test]
    async fn test_copy_paste_combination() {
        use crate::state::clipboard_data::{ClipboardState, ClipboardContent, ClipboardOperation};
        use std::io::Write;

        // Create temporary source and destination directories
        let source_dir = tempdir().expect("Failed to create source temporary directory");
        let dest_dir = tempdir().expect("Failed to create destination temporary directory");
        let intermediate_dir = tempdir().expect("Failed to create intermediate temporary directory");

        // Create a test file in the source directory
        let test_file_path = source_dir.path().join("test_file.txt");
        let mut file = fs::File::create(&test_file_path).expect("Failed to create test file");
        writeln!(file, "Test content for clipboard").expect("Failed to write to test file");
        
        // Create a test directory with content
        let test_subdir_path = source_dir.path().join("test_subdir");
        fs::create_dir(&test_subdir_path).expect("Failed to create test directory");
        let test_subfile_path = test_subdir_path.join("subfile.txt");
        let mut subfile = fs::File::create(&test_subfile_path).expect("Failed to create subfile");
        writeln!(subfile, "Content in subdirectory").expect("Failed to write to subfile");

        // Create clipboard state
        let clipboard_state = Arc::new(Mutex::new(ClipboardState::new()));

        // STEP 1: Copy file to clipboard
        let result = copy_to_clipboard(clipboard_state.clone(), test_file_path.to_str().unwrap()).await;
        assert!(result.is_ok(), "Failed to copy file to clipboard: {:?}", result);

        // STEP 2: Paste file from clipboard to intermediate directory
        let result = paste_from_clipboard(
            clipboard_state.clone(),
            intermediate_dir.path().to_str().unwrap()
        ).await;
        assert!(result.is_ok(), "Failed to paste file: {:?}", result);

        // Verify file was copied correctly
        let pasted_file_path = intermediate_dir.path().join("test_file.txt");
        assert!(pasted_file_path.exists(), "Pasted file should exist");
        let content = fs::read_to_string(&pasted_file_path).expect("Failed to read pasted file");
        assert_eq!(content, "Test content for clipboard\n", "File content should match");

        // STEP 3: Copy directory to clipboard
        let result = copy_to_clipboard(clipboard_state.clone(), test_subdir_path.to_str().unwrap()).await;
        assert!(result.is_ok(), "Failed to copy directory to clipboard: {:?}", result);

        // STEP 4: Paste directory from clipboard to destination directory
        let result = paste_from_clipboard(
            clipboard_state.clone(),
            dest_dir.path().to_str().unwrap()
        ).await;
        assert!(result.is_ok(), "Failed to paste directory: {:?}", result);

        // Verify directory was copied correctly
        let pasted_dir_path = dest_dir.path().join("test_subdir");
        let pasted_subfile_path = pasted_dir_path.join("subfile.txt");
        assert!(pasted_dir_path.exists(), "Pasted directory should exist");
        assert!(pasted_dir_path.is_dir(), "Pasted path should be a directory");
        
        let pasted_subfile_path = pasted_dir_path.join("subfile.txt");
        assert!(pasted_subfile_path.exists(), "Pasted subfile should exist");
        let content = fs::read_to_string(&pasted_subfile_path).expect("Failed to read pasted subfile");
        assert_eq!(content, "Content in subdirectory\n", "Subfile content should match");
        
        // STEP 5: Create a new clipboard with Cut operation for the previously pasted file
        let cut_clipboard = Arc::new(Mutex::new(
            ClipboardState::new_with_content(
                ClipboardContent::FilePath(pasted_file_path.clone()),
                ClipboardOperation::Cut
            )
        ));
        
        // STEP 6: Paste (move) the file to the destination directory
        let result = paste_from_clipboard(
            cut_clipboard,
            dest_dir.path().to_str().unwrap()
        ).await;
        assert!(result.is_ok(), "Failed to paste (move) file: {:?}", result);

        // Verify file was moved correctly
        assert!(!pasted_file_path.exists(), "Original file should no longer exist after cut operation");
        let final_file_path = dest_dir.path().join("test_file.txt");
        assert!(final_file_path.exists(), "Moved file should exist in destination");
        
        // Verify content was preserved after move
        let moved_content = fs::read_to_string(&final_file_path).expect("Failed to read moved file");
        assert_eq!(moved_content, "Test content for clipboard\n", "Moved file content should match");
    }

    #[tokio::test]
    async fn test_complex_clipboard_workflow() {
        use crate::state::clipboard_data::{ClipboardState, ClipboardContent, ClipboardOperation};
        use std::io::Write;

        // Create three temporary directories for our workflow
        let source_dir = tempdir().expect("Failed to create source temporary directory");
        let intermediate_dir = tempdir().expect("Failed to create intermediate temporary directory");
        let final_dir = tempdir().expect("Failed to create final temporary directory");

        // STEP 1: Setup test files and directories

        // Create a test file in source directory
        let source_file_path = source_dir.path().join("source_file.txt");
        let mut source_file = fs::File::create(&source_file_path).expect("Failed to create source file");
        writeln!(source_file, "Source file content").expect("Failed to write to source file");

        // Create a directory with nested content
        let source_subdir_path = source_dir.path().join("source_subdir");
        fs::create_dir(&source_subdir_path).expect("Failed to create source subdirectory");

        let nested_file_path = source_subdir_path.join("nested_file.txt");
        let mut nested_file = fs::File::create(&nested_file_path).expect("Failed to create nested file");
        writeln!(nested_file, "Nested file content").expect("Failed to write to nested file");

        // Create multiple files for batch operations
        let batch_file1_path = source_dir.path().join("batch_file1.txt");
        let batch_file2_path = source_dir.path().join("batch_file2.txt");
        fs::write(&batch_file1_path, "Batch file 1").expect("Failed to create batch file 1");
        fs::write(&batch_file2_path, "Batch file 2").expect("Failed to create batch file 2");

        // Create clipboard state
        let clipboard_state = Arc::new(Mutex::new(ClipboardState::new()));

        // STEP 2: Copy a single file to clipboard and paste to intermediate dir
        let result = copy_to_clipboard(clipboard_state.clone(), source_file_path.to_str().unwrap()).await;
        assert!(result.is_ok(), "Failed to copy file to clipboard");

        let result = paste_from_clipboard(
            clipboard_state.clone(),
            intermediate_dir.path().to_str().unwrap()
        ).await;
        assert!(result.is_ok(), "Failed to paste file from clipboard");

        // Verify the file was copied correctly
        let intermediate_file = intermediate_dir.path().join("source_file.txt");
        assert!(intermediate_file.exists(), "Copied file should exist in intermediate directory");
        assert!(source_file_path.exists(), "Original file should still exist in source directory");

        // STEP 3: Copy a directory to clipboard and paste to final dir
        let result = copy_to_clipboard(clipboard_state.clone(), source_subdir_path.to_str().unwrap()).await;
        assert!(result.is_ok(), "Failed to copy directory to clipboard");

        let result = paste_from_clipboard(
            clipboard_state.clone(),
            final_dir.path().to_str().unwrap()
        ).await;
        assert!(result.is_ok(), "Failed to paste directory from clipboard");

        // Verify directory structure was copied correctly
        let final_subdir = final_dir.path().join("source_subdir");
        let final_nested_file = final_subdir.join("nested_file.txt");
        assert!(final_subdir.exists() && final_subdir.is_dir(), "Copied directory should exist in final directory");
        assert!(final_nested_file.exists(), "Nested file should exist in copied directory");

        // STEP 4: Cut a file and move it
        // First create a file to cut in intermediate directory
        let cut_test_path = intermediate_dir.path().join("file_to_cut.txt");
        fs::write(&cut_test_path, "This file will be moved").expect("Failed to create file for cutting");

        // Setup clipboard with cut operation
        let cut_clipboard = Arc::new(Mutex::new(
            ClipboardState::new_with_content(
                ClipboardContent::FilePath(cut_test_path.clone()),
                ClipboardOperation::Cut
            )
        ));

        // Move the file to final directory
        let result = paste_from_clipboard(
            cut_clipboard.clone(),
            final_dir.path().to_str().unwrap()
        ).await;
        assert!(result.is_ok(), "Failed to paste (move) file");

        // Verify the file was moved
        assert!(!cut_test_path.exists(), "Original file should no longer exist after cut operation");
        let moved_file_path = final_dir.path().join("file_to_cut.txt");
        assert!(moved_file_path.exists(), "Moved file should exist in final directory");

        // STEP 5: Copy multiple files at once
        // Create a multiple items clipboard
        let multi_clipboard = Arc::new(Mutex::new(
            ClipboardState::new_with_content(
                ClipboardContent::MultipleItems(vec![batch_file1_path.clone(), batch_file2_path.clone()]),
                ClipboardOperation::Copy
            )
        ));

        // Paste multiple files to final directory
        let result = paste_from_clipboard(
            multi_clipboard.clone(),
            final_dir.path().to_str().unwrap()
        ).await;
        assert!(result.is_ok(), "Failed to paste multiple files");

        // Verify all files were copied correctly
        let final_batch_file1 = final_dir.path().join("batch_file1.txt");
        let final_batch_file2 = final_dir.path().join("batch_file2.txt");
        assert!(final_batch_file1.exists(), "Batch file 1 should exist in final directory");
        assert!(final_batch_file2.exists(), "Batch file 2 should exist in final directory");
        assert!(batch_file1_path.exists(), "Original batch file 1 should still exist after copy");
        assert!(batch_file2_path.exists(), "Original batch file 2 should still exist after copy");

        // STEP 6: Test text content in clipboard
        let text_clipboard = Arc::new(Mutex::new(
            ClipboardState::new_with_content(
                ClipboardContent::TextContent("Text from clipboard".to_string()),
                ClipboardOperation::Copy
            )
        ));

        let result = paste_from_clipboard(
            text_clipboard.clone(),
            final_dir.path().to_str().unwrap()
        ).await;
        assert!(result.is_ok(), "Failed to paste text content");

        // Verify text file was created
        let text_file_path = final_dir.path().join("clipboard_content.txt");
        assert!(text_file_path.exists(), "Text file should exist in final directory");
        let text_content = fs::read_to_string(&text_file_path).expect("Failed to read text file");
        assert_eq!(text_content, "Text from clipboard", "Text content should match");

        // Ensure all test directories remain alive until the end of the test
        drop(source_dir);
        drop(intermediate_dir);
        drop(final_dir);
    }

    #[tokio::test]
    async fn test_cut_command() {
        use crate::state::clipboard_data::{ClipboardState, ClipboardContent, ClipboardOperation};

        // Create a temporary directory
        let temp_dir = tempdir().expect("Failed to create temporary directory");

        // Create a test file
        let file_path = temp_dir.path().join("cut_test_file.txt");
        fs::write(&file_path, "Test content for cut operation").expect("Failed to create test file");

        // Create clipboard state
        let clipboard_state = Arc::new(Mutex::new(ClipboardState::new()));

        // Execute cut command
        let result = cut(clipboard_state.clone(), file_path.to_str().unwrap()).await;

        // Verify the operation was successful
        assert!(result.is_ok(), "Failed to cut file: {:?}", result);

        // Verify the clipboard content was set correctly
        let clipboard = clipboard_state.lock().unwrap();
        let clipboard_content = clipboard.get_content();

        match clipboard_content {
            ClipboardContent::FilePath(path) => {
                assert_eq!(path, file_path, "Path in clipboard doesn't match the cut file path");
            },
            _ => panic!("Wrong clipboard content type. Expected FilePath."),
        }

        // Verify clipboard operation is set to Cut
        assert_eq!(clipboard.get_operation(), ClipboardOperation::Cut);
    }

    #[tokio::test]
    async fn test_cut_directory_command() {
        use crate::state::clipboard_data::{ClipboardState, ClipboardContent, ClipboardOperation};

        // Create a temporary directory
        let temp_dir = tempdir().expect("Failed to create temporary directory");

        // Create a test subdirectory
        let dir_path = temp_dir.path().join("cut_test_dir");
        fs::create_dir(&dir_path).expect("Failed to create test directory");

        // Create some files in the directory
        let file_path = dir_path.join("test_file.txt");
        fs::write(&file_path, "Test content in directory").expect("Failed to create test file");

        // Create clipboard state
        let clipboard_state = Arc::new(Mutex::new(ClipboardState::new()));

        // Execute cut command for directory
        let result = cut(clipboard_state.clone(), dir_path.to_str().unwrap()).await;

        // Verify the operation was successful
        assert!(result.is_ok(), "Failed to cut directory: {:?}", result);

        // Verify the clipboard content was set correctly
        let clipboard = clipboard_state.lock().unwrap();
        let clipboard_content = clipboard.get_content();

        match clipboard_content {
            ClipboardContent::FolderPath(path) => {
                assert_eq!(path, dir_path, "Path in clipboard doesn't match the cut directory path");
            },
            _ => panic!("Wrong clipboard content type. Expected FolderPath."),
        }

        // Verify clipboard operation is set to Cut
        assert_eq!(clipboard.get_operation(), ClipboardOperation::Cut);
    }

    #[tokio::test]
    async fn test_cut_nonexistent_path() {
        use crate::state::clipboard_data::ClipboardState;

        // Create a non-existent path
        let non_existent_path = "/path/that/does/not/exist/file.txt";

        // Create clipboard state
        let clipboard_state = Arc::new(Mutex::new(ClipboardState::new()));

        // Execute cut command with non-existent path
        let result = cut(clipboard_state, non_existent_path).await;

        // Verify the operation fails with appropriate error
        assert!(result.is_err(), "Expected an error when cutting a non-existent path");
        assert!(result.unwrap_err().contains("Path does not exist"));
    }

    #[tokio::test]
    async fn test_cut_multiple_items_command() {
        use crate::state::clipboard_data::{ClipboardState, ClipboardContent, ClipboardOperation};

        // Create a temporary directory
        let temp_dir = tempdir().expect("Failed to create temporary directory");

        // Create multiple test files
        let file1_path = temp_dir.path().join("cut_test_file1.txt");
        let file2_path = temp_dir.path().join("cut_test_file2.txt");
        fs::write(&file1_path, "Test content for first file").expect("Failed to create first test file");
        fs::write(&file2_path, "Test content for second file").expect("Failed to create second test file");

        // Create clipboard state
        let clipboard_state = Arc::new(Mutex::new(ClipboardState::new()));

        // Execute cut_multiple_items command
        let paths_vec = vec![
            file1_path.to_str().unwrap().to_string(),
            file2_path.to_str().unwrap().to_string()
        ];

        let result = cut_multiple_items(clipboard_state.clone(), paths_vec).await;

        // Verify the operation was successful
        assert!(result.is_ok(), "Failed to cut multiple items: {:?}", result);

        // Verify the clipboard content was set correctly
        let clipboard = clipboard_state.lock().unwrap();
        let clipboard_content = clipboard.get_content();

        match clipboard_content {
            ClipboardContent::MultipleItems(paths) => {
                assert_eq!(paths.len(), 2, "Expected two paths in clipboard");
                assert!(paths.contains(&file1_path), "First file path missing from clipboard");
                assert!(paths.contains(&file2_path), "Second file path missing from clipboard");
            },
            _ => panic!("Wrong clipboard content type. Expected MultipleItems."),
        }

        // Verify clipboard operation is set to Cut
        assert_eq!(clipboard.get_operation(), ClipboardOperation::Cut);
    }

    #[tokio::test]
    async fn test_cut_paste_workflow() {
        use crate::state::clipboard_data::ClipboardState,;

        // Create source and destination directories
        let source_dir = tempdir().expect("Failed to create source directory");
        let dest_dir = tempdir().expect("Failed to create destination directory");

        // Create test file in source directory
        let file_path = source_dir.path().join("cut_paste_test.txt");
        fs::write(&file_path, "Content for cut-paste test").expect("Failed to create test file");

        // Create test directory in source directory
        let dir_path = source_dir.path().join("cut_paste_dir");
        fs::create_dir(&dir_path).expect("Failed to create test directory");

        // Create file inside test directory
        let nested_file_path = dir_path.join("nested.txt");
        fs::write(&nested_file_path, "Nested file content").expect("Failed to create nested file");

        // Verify files exist in source location
        assert!(file_path.exists(), "Source file should exist before cut");
        assert!(dir_path.exists(), "Source directory should exist before cut");
        assert!(nested_file_path.exists(), "Nested file should exist before cut");

        // Create clipboard state
        let clipboard_state = Arc::new(Mutex::new(ClipboardState::new()));

        // STEP 1: Cut the file and paste to destination
        let result = cut(clipboard_state.clone(), file_path.to_str().unwrap()).await;
        assert!(result.is_ok(), "Failed to cut file: {:?}", result);

        let result = paste_from_clipboard(
            clipboard_state.clone(),
            dest_dir.path().to_str().unwrap()
        ).await;
        assert!(result.is_ok(), "Failed to paste cut file: {:?}", result);

        // Verify file was moved (not in source, exists in destination)
        assert!(!file_path.exists(), "Source file should no longer exist after cut-paste");
        let dest_file_path = dest_dir.path().join("cut_paste_test.txt");
        assert!(dest_file_path.exists(), "Destination file should exist after cut-paste");

        let content = fs::read_to_string(&dest_file_path).expect("Failed to read destination file");
        assert_eq!(content, "Content for cut-paste test", "File content should match after cut-paste");

        // STEP 2: Cut the directory and paste to destination
        let result = cut(clipboard_state.clone(), dir_path.to_str().unwrap()).await;
        assert!(result.is_ok(), "Failed to cut directory: {:?}", result);

        let result = paste_from_clipboard(
            clipboard_state.clone(),
            dest_dir.path().to_str().unwrap()
        ).await;
        assert!(result.is_ok(), "Failed to paste cut directory: {:?}", result);

        // Verify directory was moved
        assert!(!dir_path.exists(), "Source directory should no longer exist after cut-paste");
        assert!(!nested_file_path.exists(), "Nested file should no longer exist at source after cut-paste");

        let dest_dir_path = dest_dir.path().join("cut_paste_dir");
        let dest_nested_file_path = dest_dir_path.join("nested.txt");
        assert!(dest_dir_path.exists() && dest_dir_path.is_dir(), "Destination directory should exist after cut-paste");
        assert!(dest_nested_file_path.exists(), "Nested file should exist at destination after cut-paste");

        let nested_content = fs::read_to_string(&dest_nested_file_path).expect("Failed to read nested file");
        assert_eq!(nested_content, "Nested file content", "Nested file content should match after cut-paste");
    }

    #[tokio::test]
    async fn test_cut_multiple_and_paste_workflow() {
        use crate::state::clipboard_data::{ClipboardState, ClipboardContent, ClipboardOperation};

        // Create source and destination directories
        let source_dir = tempdir().expect("Failed to create source directory");
        let dest_dir = tempdir().expect("Failed to create destination directory");

        // Create multiple test files in source directory
        let file1_path = source_dir.path().join("multi_cut1.txt");
        let file2_path = source_dir.path().join("multi_cut2.txt");
        let file3_path = source_dir.path().join("not_cut.txt");

        fs::write(&file1_path, "Content for multi-cut 1").expect("Failed to create test file 1");
        fs::write(&file2_path, "Content for multi-cut 2").expect("Failed to create test file 2");
        fs::write(&file3_path, "Content that shouldn't be cut").expect("Failed to create test file 3");

        // Create a directory to cut
        let dir_path = source_dir.path().join("multi_cut_dir");
        fs::create_dir(&dir_path).expect("Failed to create test directory");

        // Verify files and directory exist in source location
        assert!(file1_path.exists(), "First file should exist before cut");
        assert!(file2_path.exists(), "Second file should exist before cut");
        assert!(file3_path.exists(), "Third file should exist before cut");
        assert!(dir_path.exists(), "Directory should exist before cut");

        // Create clipboard state
        let clipboard_state = Arc::new(Mutex::new(ClipboardState::new()));

        // Cut multiple items - two files and one directory
        let paths_vec = vec![
            file1_path.to_str().unwrap().to_string(),
            file2_path.to_str().unwrap().to_string(),
            dir_path.to_str().unwrap().to_string()
        ];

        let result = cut_multiple_items(clipboard_state.clone(), paths_vec).await;
        assert!(result.is_ok(), "Failed to cut multiple items: {:?}", result);

        // Paste to destination
        let result = paste_from_clipboard(
            clipboard_state.clone(),
            dest_dir.path().to_str().unwrap()
        ).await;
        assert!(result.is_ok(), "Failed to paste multiple cut items: {:?}", result);

        // Verify first file was moved
        assert!(!file1_path.exists(), "First file should no longer exist after cut-paste");
        let dest_file1_path = dest_dir.path().join("multi_cut1.txt");
        assert!(dest_file1_path.exists(), "First file should exist at destination after cut-paste");
        let content1 = fs::read_to_string(&dest_file1_path).expect("Failed to read first destination file");
        assert_eq!(content1, "Content for multi-cut 1", "First file content should match after cut-paste");

        // Verify second file was moved
        assert!(!file2_path.exists(), "Second file should no longer exist after cut-paste");
        let dest_file2_path = dest_dir.path().join("multi_cut2.txt");
        assert!(dest_file2_path.exists(), "Second file should exist at destination after cut-paste");
        let content2 = fs::read_to_string(&dest_file2_path).expect("Failed to read second destination file");
        assert_eq!(content2, "Content for multi-cut 2", "Second file content should match after cut-paste");

        // Verify directory was moved
        assert!(!dir_path.exists(), "Directory should no longer exist after cut-paste");
        let dest_dir_path = dest_dir.path().join("multi_cut_dir");
        assert!(dest_dir_path.exists() && dest_dir_path.is_dir(), "Directory should exist at destination after cut-paste");

        // Verify third file was NOT moved (as it wasn't included in the cut operation)
        assert!(file3_path.exists(), "Third file should still exist at source after cut-paste");
        let dest_file3_path = dest_dir.path().join("not_cut.txt");
        assert!(!dest_file3_path.exists(), "Third file should NOT exist at destination after cut-paste");
    }
}

