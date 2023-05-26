# Kushn: SHA256 File Hash Generator

## Table of Contents

- [Overview](#overview)
- [Features](#features)
- [Installation](#installation)
- [Usage](#usage)
- [Kushnignore](#kushnignore)
- [Output](#output)
- [Contributions](#contributions)
- [License](#license)

---

## Overview

**Kushn** is a robust and lightweight utility written in Rust for generating SHA256 hashes of files. <br />
It recursively scans all files in the current directory and its subdirectories, creating a JSON file. <br />
This file provides a clear overview of each file and its corresponding hash.

---

## Features

- **Hashing:** Generates SHA256 hashes for all files in the current directory and nested directories.
- **Customizable Output:** Allows specification of a custom output file name.
- **Ignores Files or Folders:** Supports the use of a `.kushnignore` file to specify files, folders, or file types to be excluded from the scan.

---

## Installation

Install Kushn directly from the official Rust package manager, cargo:

```bash
cargo install kushn
```

---

## Usage

Run Kushn in the current directory:

```bash
kushn
```

To specify a custom output file name, use:

```bash
kushn --name your_name.json
```

---

## Kushnignore

To ignore specific files, folders, nested folders, or file types during the scan, create a `.kushnignore` file in the root directory.

- Ignore a folder: `folder`
- Ignore a nested folder: `folder/subfolder`
- Ignore a specific file type: `*.txt`
- Ignore a specific file: `test.txt` or `folder/test.txt`

---

## Output

The output will be a JSON file (`kushn_result.json` by default, or a custom name if specified) containing an array of objects.  <br />
Each object represents a file and its hash.

Example output:

```json
[
  {
    "path": "folder/test.txt",
    "hash": "12345"
  }
]
```

---

## Contributions

Contributions, issues, and feature requests are welcome.

---

## License

Distributed under the MIT License. See `LICENSE` for more information.


