#!/usr/bin/env python3
"""
crate_recreator.py

This script generates a self-contained Python script that, when executed, recreates 
the directory structure and file contents of a Rust crate. The process is as follows:

1. Starting from a specified source folder (or using --src-only to restrict to its "src" subfolder),
   the script searches upward for a Cargo.toml file. When found, it uses the folder containing 
   Cargo.toml as the crate root.
2. It extracts the crate name from the Cargo.toml’s [package] section. If not found, it falls back 
   to using the basename of the originally provided folder.
3. The script then gathers all files recursively from the determined root (or root/src if --src-only),
   ignoring directories like ".git", "target", ".aipack", ".github" and files such as ".gitignore", 
   "Cargo.lock", or any file whose name starts with "LICENSE" or "NOTICE", as well as binary files 
   (e.g., .webp, .jpg, .jpeg, .png).
4. It generates a self-contained Python script that will recreate the crate’s structure and embedded file contents.
   In the generated script each embedded file is annotated with a comment showing its relative path.
5. The generated file is named in the format: <crate_name>_recreate_YYMMDD_HHMM.py.
6. The generated script is set as executable and its content is copied to the clipboard.
   (This copied content is the generated recreate script—not this crate recreator.)

Usage:
    python crate_recreator.py <source_folder> [--src-only]
"""

import os
import sys
import argparse
import subprocess
import stat
from datetime import datetime
import re

def gather_files(source_folder):
    """
    Walk the source folder recursively and return a dictionary mapping
    relative file paths to their contents.

    Excludes directories named "target" (case-insensitive), ".git", ".aipack", ".github",
    and files such as ".gitignore", "Cargo.lock", or any file whose name starts with 
    "LICENSE" or "NOTICE". Also ignores binary files with extensions like .webp, .jpg, .jpeg, .png.

    Provides detailed tracing for debugging.
    """
    ignore_dirs = {".git", ".aipack", ".github"}
    ignore_files = {".gitignore", "Cargo.lock"}
    binary_extensions = (".webp", ".jpg", ".jpeg", ".png")
    files_dict = {}
    print(f"[TRACE] Starting to traverse source folder: {source_folder}")
    for root, dirs, files in os.walk(source_folder):
        # Exclude specified directories.
        original_dirs = dirs.copy()
        dirs[:] = [d for d in dirs if d.lower() != "target" and d not in ignore_dirs]
        for excluded in set(original_dirs) - set(dirs):
            print(f"[TRACE] Excluding directory: {excluded}")
        # Process each file in the current directory.
        for file in files:
            # Check ignore conditions for file names.
            if file in ignore_files  or file.endswith(('.bak', '~')):
                print(f"[TRACE] Ignoring file: {file}")
                continue
            if file.startswith("LICENSE") or file.startswith("NOTICE"):
                print(f"[TRACE] Ignoring file (starts with LICENSE or NOTICE): {file}")
                continue
            lower_file = file.lower()
            if lower_file.endswith(binary_extensions):
                print(f"[TRACE] Ignoring binary file: {file}")
                continue

            full_path = os.path.join(root, file)
            # Compute relative path based on the source folder.
            rel_path = os.path.relpath(full_path, source_folder)
            print(f"[TRACE] Processing file: {full_path} as {rel_path}")
            try:
                with open(full_path, "r", encoding="utf-8") as f:
                    content = f.read()
            except Exception as e:
                print(f"[WARNING] Skipping file {full_path} due to read error: {e}")
                continue
            files_dict[rel_path] = content
    print(f"[TRACE] Completed traversing. Total files gathered: {len(files_dict)}")
    return files_dict

def generate_script(files_dict, crate_name):
    """
    Generate a self-contained Python script as a string.
    When run, the generated script creates a folder (named after the crate)
    and reconstructs all files with their original contents.

    This generated script also includes:
      - A function to copy its own source code to the clipboard.
      - Detailed tracing messages to follow its execution.

    Each embedded file is annotated with a trailing comment indicating its relative path.
    
    Returns:
        A string containing the full source code of the generated script.
    """
    lines = []
    lines.append("#!/usr/bin/env python3")
    lines.append("import os")
    lines.append("import sys")
    lines.append("import subprocess")
    lines.append("")
    lines.append("def copy_to_clipboard(text):")
    lines.append("    \"\"\"")
    lines.append("    Copies the given text to the system clipboard.")
    lines.append("    Uses 'clip' on Windows and 'pbcopy' on macOS.")
    lines.append("    \"\"\"")
    lines.append("    try:")
    lines.append("        if sys.platform.startswith('win'):")
    lines.append("            proc = subprocess.Popen(['clip'], stdin=subprocess.PIPE, close_fds=True)")
    lines.append("            proc.communicate(input=text.encode('utf-8'))")
    lines.append("        elif sys.platform == 'darwin':")
    lines.append("            proc = subprocess.Popen(['pbcopy'], stdin=subprocess.PIPE, close_fds=True)")
    lines.append("            proc.communicate(input=text.encode('utf-8'))")
    lines.append("        else:")
    lines.append("            print('[TRACE] Clipboard copy not supported on this platform.')")
    lines.append("    except Exception as e:")
    lines.append("        print(f'[ERROR] Failed to copy to clipboard: {e}')")
    lines.append("")
    lines.append("def copy_self_to_clipboard():")
    lines.append("    \"\"\"")
    lines.append("    Reads its own source file and copies the content to the clipboard.")
    lines.append("    Provides detailed tracing for debugging.")
    lines.append("    \"\"\"")
    lines.append("    try:")
    lines.append("        with open(__file__, 'r', encoding='utf-8') as f:")
    lines.append("            content = f.read()")
    lines.append("        copy_to_clipboard(content)")
    lines.append("        print('[TRACE] The script has been copied to the clipboard.')")
    lines.append("    except Exception as e:")
    lines.append("        print(f'[ERROR] Failed to copy self to clipboard: {e}')")
    lines.append("")
    lines.append("def create_crate():")
    lines.append("    \"\"\"")
    lines.append("    Recreates the directory structure and files for the crate.")
    lines.append("    Provides detailed tracing for each step.")
    lines.append("    \"\"\"")
    lines.append(f"    base_folder = os.path.join(os.getcwd(), '{crate_name}')")
    lines.append("    print(f'[TRACE] Creating base folder: {base_folder}')")
    lines.append("    os.makedirs(base_folder, exist_ok=True)")
    lines.append("    files = {")
    # Embed each file's content along with a comment indicating its relative path.
    for path, content in files_dict.items():
        lines.append(f"        {repr(path)}: {repr(content)},  # File: {path}")
    lines.append("    }")
    lines.append("")
    lines.append("    for relative_path, content in files.items():")
    lines.append("        full_path = os.path.join(base_folder, relative_path)")
    lines.append("        directory = os.path.dirname(full_path)")
    lines.append("        if not os.path.exists(directory):")
    lines.append("            os.makedirs(directory, exist_ok=True)")
    lines.append("            print(f'[TRACE] Created directory: {directory}')")
    lines.append("        with open(full_path, 'w', encoding='utf-8') as f:")
    lines.append("            f.write(content)")
    lines.append("        print(f'[TRACE] Created file: {full_path}')")
    lines.append("")
    lines.append("if __name__ == '__main__':")
    lines.append("    create_crate()")
    lines.append("    # Uncomment the next line to enable self-copy functionality.")
    lines.append("    # copy_self_to_clipboard()")
    lines.append("    print('[TRACE] Crate creation complete.')")
    
    return "\n".join(lines)

def copy_to_clipboard(text):
    """
    Copies the given text to the system clipboard.
    Uses 'clip' on Windows and 'pbcopy' on macOS.
    Provides tracing information.
    """
    try:
        if sys.platform.startswith("win"):
            print("[TRACE] Using 'clip' for clipboard copy on Windows.")
            proc = subprocess.Popen(["clip"], stdin=subprocess.PIPE, close_fds=True)
            proc.communicate(input=text.encode("utf-8"))
        elif sys.platform == "darwin":
            print("[TRACE] Using 'pbcopy' for clipboard copy on macOS.")
            proc = subprocess.Popen(["pbcopy"], stdin=subprocess.PIPE, close_fds=True)
            proc.communicate(input=text.encode("utf-8"))
        else:
            print("[TRACE] Clipboard copy not supported on this platform.")
    except Exception as e:
        print(f"[ERROR] Failed to copy to clipboard: {e}")

def find_cargo_toml(start_dir):
    """
    Starting from start_dir, search upward for a Cargo.toml file.
    Returns the path to Cargo.toml if found, otherwise None.
    """
    current_dir = start_dir
    while True:
        candidate = os.path.join(current_dir, "Cargo.toml")
        if os.path.exists(candidate):
            print(f"[TRACE] Found Cargo.toml at: {candidate}")
            return candidate
        parent = os.path.dirname(current_dir)
        if parent == current_dir:  # Reached root directory
            break
        current_dir = parent
    print("[TRACE] No Cargo.toml found in the directory hierarchy.")
    return None

def get_crate_name_from_cargo_toml(cargo_toml_path):
    """
    Parse the Cargo.toml file to extract the crate name from the [package] section.
    Returns the crate name if found, otherwise None.
    """
    in_package = False
    try:
        with open(cargo_toml_path, "r", encoding="utf-8") as f:
            for line in f:
                stripped = line.strip()
                if stripped.startswith("[package]"):
                    in_package = True
                elif stripped.startswith("[") and in_package:
                    # Exiting the [package] section.
                    break
                elif in_package and stripped.startswith("name"):
                    match = re.search(r'name\s*=\s*["\'](.+?)["\']', stripped)
                    if match:
                        crate_name = match.group(1)
                        print(f"[TRACE] Crate name found in Cargo.toml: {crate_name}")
                        return crate_name
    except Exception as e:
        print(f"[ERROR] Failed to parse Cargo.toml: {e}")
    return None

def main():
    parser = argparse.ArgumentParser(
        description="Generate a self-contained Python script that recreates a Rust crate with enhanced features."
    )
    parser.add_argument("source_folder", help="Path to the source folder (should be within or at the crate root).")
    parser.add_argument("--src-only", action="store_true", help="Process only the 'src' folder inside the crate root.")
    args = parser.parse_args()

    # Resolve the absolute path of the provided source folder.
    orig_source_folder = os.path.abspath(args.source_folder)
    print(f"[TRACE] Source folder resolved to: {orig_source_folder}")

    # Search upward for Cargo.toml from the provided folder.
    cargo_toml_path = find_cargo_toml(orig_source_folder)
    if cargo_toml_path:
        # Use the directory containing Cargo.toml as the crate root.
        crate_root = os.path.dirname(cargo_toml_path)
        crate_name_from_toml = get_crate_name_from_cargo_toml(cargo_toml_path)
        if crate_name_from_toml:
            crate_name = crate_name_from_toml
            print(f"[TRACE] Using crate name from Cargo.toml: {crate_name}")
        else:
            crate_name = os.path.basename(orig_source_folder.rstrip(os.sep))
            print("[TRACE] Cargo.toml found but crate name could not be extracted; using fallback name.")
    else:
        crate_root = orig_source_folder
        crate_name = os.path.basename(orig_source_folder.rstrip(os.sep))
        print("[TRACE] No Cargo.toml found; using fallback crate name.")

    # Determine the folder to gather files from.
    if args.src_only:
        print("[TRACE] --src-only flag is set; processing only the 'src' subfolder.")
        source_folder = os.path.join(crate_root, "src")
    else:
        source_folder = crate_root

    if not os.path.exists(source_folder):
        print(f"[ERROR] Source folder '{source_folder}' does not exist.")
        sys.exit(1)

    # Gather files from the determined source folder.
    files_dict = gather_files(source_folder)

    # Generate the script content (this is the recreate script that will be copied to clipboard).
    generated_script = generate_script(files_dict, crate_name)

    # Create a timestamped output file name in the format: <crate_name>_recreate_YYMMDD_HHMM.py
    timestamp = datetime.now().strftime("%y%m%d_%H%M")
    output_file = f"{crate_name}_recreate_{timestamp}.py"
    print(f"[TRACE] Writing generated script to: {output_file}")
    with open(output_file, "w", encoding="utf-8") as f:
        f.write(generated_script)
    
    # Set the generated script to be executable.
    try:
        st = os.stat(output_file)
        os.chmod(output_file, st.st_mode | stat.S_IEXEC)
        print(f"[TRACE] Set executable permission for {output_file}.")
    except Exception as e:
        print(f"[ERROR] Failed to set executable permission for {output_file}: {e}")

    print(f"[TRACE] Generated script saved to {output_file}.")

    # Copy the generated recreate script's content to the clipboard.
    # (This is the script that, when run, will recreate the crate.)
    copy_to_clipboard(generated_script)
    print("[TRACE] Generated script copied to clipboard.")

if __name__ == "__main__":
    main()

