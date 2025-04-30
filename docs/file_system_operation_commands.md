# Tauri Filesystem Commands Documentation

## Content
- [Open a File](#open_file-endpoint)
- [Create a File](#create_file-endpoint)
- [Open a Directory](#open_directory-endpoint)
- [Create a Directory](#create_directory-endpoint)
- [Rename a Dir or File](#rename-endpoint)
- [Move a Dir or File to trash](#move_to_trash-endpoint)
- [Zip a Dir or File](#zip-endpoint)
- [Unzip a Dir or File](#unzip-endpoint)
- [Paste from Clipboard](#paste_from_clipboard-endpoint)
- [Copy to Clipboard](#copy_to_clipboard-endpoint)
- [Cut](#cut-endpoint)
- [Cut Multiple Items](#cut_multiple_items-endpoint)


# `open_file` endpoint

---
## Parameters
- `file_path`: The path to the file to be opened. This should be a string representing the absolute path to the file.
## Returns
- Ok(String) - The content of a file as a string.
- Err(String) - An error message if the file cannot be opened or other errors occur.

## Example call
```typescript jsx
useEffect(() => {
    const fetchMetaData = async () => {
        try {
            const result = await invoke("open_file", { file_path: "/path/to/file" });
            console.log("Fetched MetaData:", result);
        } catch (error) {
            console.error("Error fetching metadata:", error);
        }
    };

    fetchMetaData();
}, []);
```

# `create_file` endpoint

---
## Parameters
- `folder_path_abs`: The absolute path to the folder where the file will be created.
- `file_name`: The name of the file to be created. This should be a string representing the name of the file.

## Returns
- Ok(): No content is returned. The function will create a file at the specified path.
- Err(String) - An error message if the file cannot be created or other errors occur.

# `open_directory` endpoint

---
- `path`: The path to the directory to be opened. This should be a string representing the absolute path to the directory.

## Returns
- Ok(String) - A JSON string representing the contents of the directory. The structure is:
```json
  {
    "directories": [
      {
        "name": "subdir",
        "path": "/path/to/subdir",
        "is_symlink": false,
        "access_rights_as_string": "rwxr-xr-x",
        "access_rights_as_number": 16877,
        "size_in_bytes": 38,
        "sub_file_count": 2,
        "sub_dir_count": 1,
        "created": "2023-04-13 19:34:14",
        "last_modified": "2023-04-13 19:34:14",
        "accessed": "2023-04-13 19:34:14"
      }
    ],
    "files": [
      {
        "name": "file1.txt",
        "path": "/path/to/file1.txt",
        "is_symlink": false,
        "access_rights_as_string": "rw-r--r--",
        "access_rights_as_number": 33188,
        "size_in_bytes": 15,
        "created": "2023-04-13 19:34:14",
        "last_modified": "2023-04-13 19:34:14",
        "accessed": "2023-04-13 19:34:14"
      }
    ]
  }
```

# `create_directory` endpoint

---
## Parameters

- `folder_path_abs`: The absolute path to the folder where the directory will be created.
- `directory_name`: The name of the directory to be created. This should be a string representing the name of the directory.

## Returns
- Ok(): No content is returned. The function will create a directory at the specified path.
- Err(String) - An error message if the directory cannot be created or other errors occur.

# `rename` endpoint

---
## Parameters
- `old_path`: The current path of the file or directory to be renamed. This should be a string representing the absolute path.
- `new_path`: The new path for the file or directory. This should be a string representing the new absolute path.

## Returns
- Ok(): No content is returned. The function will rename the file or directory at the specified path.
- Err(String) - An error message if the file or directory cannot be renamed or other errors occur.

# `move_to_trash` endpoint

---
## Parameters
- `path`: The path to the file or directory to be moved to the trash. This should be a string representing the absolute path.

## Returns
- Ok(): No content is returned. The function will move the file or directory to the trash.
- Err(String) - An error message if the file or directory cannot be moved to the trash or other errors occur.

# `zip` endpoint

---
## Parameters
- `source_path(s)`: An array of paths to files and/or directories to be zipped. Each path should be a string representing the absolute path.
- `destination_path`: An optional destination path for the zip file. Required when zipping multiple files/directories. When not provided for a single source, creates a zip with the same name as the source.

## Returns
- Ok(): No content is returned. The function will create a zip file at the specified or default location.
- Err(String) - An error message if the zip operation fails.

## Description
Creates a zip archive from one or more files/directories. For a single source with no destination specified, creates a zip file at the same location with the same name. When zipping multiple sources or when specifying a destination, creates the zip at the specified location. All directory contents including subdirectories are included in the zip.

## Example call
```typescript jsx
useEffect(() => {
    const zipFiles = async () => {
        try {
            // Single file with auto destination
            await invoke("zip", { 
                source_paths: ["/path/to/file"],
                destination_path: null 
            });
            
            // Multiple files with specified destination
            await invoke("zip", { 
                source_paths: ["/path/to/file1", "/path/to/dir1"],
                destination_path: "/path/to/archive.zip"
            });
        } catch (error) {
            console.error("Error creating zip:", error);
        }
    };

    zipFiles();
}, []);
```

# `unzip` endpoint

---
## Parameters
- `zip_path(s)`: An array of paths to zip files to be extracted. Each path should be a string representing the absolute path.
- `destination_path`: An optional destination directory for extraction. Required when extracting multiple zips. When not provided for a single zip, extracts to a directory with the same name as the zip file (without .zip extension).

## Returns
- Ok(): No content is returned. The function will extract all zip files to the specified or default location.
- Err(String) - An error message if any extraction fails.

## Description
Extracts one or more zip files. For a single zip without a specified destination, creates a directory with the same name as the zip file (without .zip extension) and extracts contents there. When extracting multiple zips or specifying a destination, creates subdirectories for each zip under the destination path using the zip filenames. Preserves the internal directory structure of the zip files.

## Example call
```typescript jsx
useEffect(() => {
    const unzip = async () => {
        try {
            // Single zip with auto destination
            await invoke("unzip", { 
                zip_paths: ["/path/to/archive.zip"],
                destination_path: null
            });
            
            // Multiple zips with specified destination
            await invoke("unzip", { 
                zip_paths: [
                    "/path/to/archive1.zip",
                    "/path/to/archive2.zip"
                ],
                destination_path: "/path/to/extract"
            });
        } catch (error) {
            console.error("Error extracting zips:", error);
        }
    };

    unzip();
}, []);
```

# `paste_from_clipboard` endpoint

---
## Parameters
- `destination_path`: The path to the directory where the clipboard content will be pasted. This should be a string representing the absolute path.

## Returns
- Ok(): No content is returned. The function will paste the clipboard content to the specified destination.
- Err(String) - An error message if the clipboard content cannot be pasted or other errors occur.

## Description
Pastes the current contents of the clipboard to the specified location. If the clipboard contains a file or directory path, it will be copied or moved (depending on whether it was copied or cut) to the destination. If the clipboard contains text content, it will be saved as a text file. The command supports both single and multiple items in the clipboard.

## Example call
```typescript jsx
useEffect(() => {
    const pasteContent = async () => {
        try {
            await invoke("paste_from_clipboard", { 
                destination_path: "/path/to/destination" 
            });
            console.log("Content pasted successfully");
        } catch (error) {
            console.error("Error pasting content:", error);
        }
    };

    pasteContent();
}, []);
```

# `copy_to_clipboard` endpoint

---
## Parameters
- `path`: The path to the file or directory to be copied to clipboard. This should be a string representing the absolute path.

## Returns
- Ok(): No content is returned. The function will copy the path to both the application's internal clipboard and the system clipboard if possible.
- Err(String) - An error message if the path cannot be copied or other errors occur.

## Description
Copies a file or directory path to the clipboard for later pasting. The path is made available in both the application's internal clipboard and the system clipboard when possible. This operation allows the user to later use the `paste_from_clipboard` command to copy or move the file to a new location.

## Example call
```typescript jsx
useEffect(() => {
    const copyToClipboard = async () => {
        try {
            await invoke("copy_to_clipboard", { 
                path: "/path/to/file.txt" 
            });
            console.log("Path copied to clipboard successfully");
        } catch (error) {
            console.error("Error copying to clipboard:", error);
        }
    };

    copyToClipboard();
}, []);
```

# `cut` endpoint

---
## Parameters
- `path`: The path to the file or directory to be cut. This should be a string representing the absolute path.

## Returns
- Ok(): No content is returned. The function will mark the path for cutting in the clipboard.
- Err(String) - An error message if the path cannot be cut or other errors occur.

## Description
Marks a file or directory for cutting (moving) to the clipboard. When the `paste_from_clipboard` command is later used, the item will be moved to the destination location instead of being copied. This is the functional equivalent of the "Cut" operation in traditional file managers.

## Example call
```typescript jsx
useEffect(() => {
    const cutItem = async () => {
        try {
            await invoke("cut", { 
                path: "/path/to/file.txt" 
            });
            console.log("Path marked for cutting");
        } catch (error) {
            console.error("Error cutting path:", error);
        }
    };
    cutItem();
}, []);
 ````           
            
# `cut_multiple_items` endpoint

---
## Parameters
- `paths`: The paths to the files or directories to be cut. This should be a Vector representing the absolute paths.

## Returns
- Ok(): No content is returned. The function will mark the paths for cutting in the clipboard.
- Err(String) - An error message if the paths cannot be cut or other errors occur.

## Description
Marks files or directories for cutting (moving) to the clipboard. When the `paste_from_clipboard` command is later used, the item will be moved to the destination location instead of being copied. This is the functional equivalent of the "Cut" operation in traditional file managers.

## Example call
```typescript jsx
useEffect(() => {
    const cutItems = async () => {
        try {
            await invoke("cut_multiple_items", { 
                paths: ["/path/to/file.txt", "/second_path/to_other/folder"] 
            });
            console.log("Paths marked for cutting");
        } catch (error) {
            console.error("Error cutting paths:", error);
        }
    };
    cutItems();
}, []);
```