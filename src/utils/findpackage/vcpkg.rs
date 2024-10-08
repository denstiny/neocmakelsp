use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use std::sync::LazyLock;

use crate::utils::{CMakePackage, FileType};

use super::{get_version, CMAKECONFIG, CMAKECONFIGVERSION, CMAKEREGEX};

pub fn did_vcpkg_project(path: &Path) -> bool {
    if path.is_dir() && path.join("vcpkg.json").is_file() {
        return true;
    }
    false
}

pub static VCPKG_PREFIX: LazyLock<Arc<Mutex<Vec<&str>>>> =
    LazyLock::new(|| Arc::new(Mutex::new([].to_vec())));

pub static VCPKG_LIBS: LazyLock<Arc<Mutex<Vec<&str>>>> =
    LazyLock::new(|| Arc::new(Mutex::new([].to_vec())));

fn get_available_libs() -> Vec<PathBuf> {
    let mut ava: Vec<PathBuf> = vec![];
    let vcpkg_prefix = VCPKG_PREFIX.lock().unwrap();
    let vcpkg_libs = VCPKG_LIBS.lock().unwrap();
    for prefix in vcpkg_prefix.iter() {
        for lib in vcpkg_libs.iter() {
            let p = Path::new(prefix).join(lib);
            if p.exists() {
                ava.push(p);
            }
        }
    }
    ava
}

fn get_cmake_message() -> HashMap<String, CMakePackage> {
    let mut packages: HashMap<String, CMakePackage> = HashMap::new();
    let vcpkg_prefix = VCPKG_PREFIX.lock().unwrap();
    for lib in vcpkg_prefix.iter() {
        let Ok(paths) = glob::glob(&format!("{lib}/share/*/cmake/")) else {
            continue;
        };
        for path in paths.flatten() {
            let Ok(files) = glob::glob(&format!("{}/*.cmake", path.to_string_lossy())) else {
                continue;
            };
            let mut tojump: Vec<PathBuf> = vec![];
            let mut version: Option<String> = None;
            let mut ispackage = false;
            for f in files.flatten() {
                tojump.push(fs::canonicalize(f.clone()).unwrap());
                if CMAKECONFIG.is_match(f.to_str().unwrap()) {
                    ispackage = true;
                }
                if CMAKECONFIGVERSION.is_match(f.to_str().unwrap()) {
                    if let Ok(context) = fs::read_to_string(&f) {
                        version = get_version(&context);
                    }
                }
            }
            if ispackage {
                let packagename = path
                    .parent()
                    .unwrap()
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap();
                packages
                    .entry(packagename.to_string())
                    .or_insert_with(|| CMakePackage {
                        name: packagename.to_string(),
                        filetype: FileType::Dir,
                        filepath: path.to_str().unwrap().to_string(),
                        version,
                        tojump,
                        from: "Vcpkg".to_string(),
                    });

                //ava.push(path);
            }
        }
    }
    drop(vcpkg_prefix);
    for lib in get_available_libs() {
        let Ok(paths) = std::fs::read_dir(lib) else {
            continue;
        };
        for path in paths.flatten() {
            let mut version: Option<String> = None;
            let mut tojump: Vec<PathBuf> = vec![];
            let pathname = path.file_name().to_str().unwrap().to_string();
            let packagepath = path.path().to_str().unwrap().to_string();
            let (packagetype, packagename) = {
                if path.metadata().unwrap().is_dir() {
                    if let Ok(paths) = std::fs::read_dir(path.path().to_str().unwrap()) {
                        for path in paths.flatten() {
                            let filepath = fs::canonicalize(path.path()).unwrap();
                            if path.metadata().unwrap().is_file() {
                                let filename = path.file_name().to_str().unwrap().to_string();
                                if CMAKEREGEX.is_match(&filename) {
                                    tojump.push(filepath.clone());
                                    if CMAKECONFIGVERSION.is_match(&filename) {
                                        if let Ok(context) = fs::read_to_string(&filepath) {
                                            version = get_version(&context);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    (FileType::Dir, pathname)
                } else {
                    let filepath = fs::canonicalize(path.path()).unwrap();
                    tojump.push(filepath);
                    let pathname = pathname.split('.').collect::<Vec<&str>>()[0].to_string();
                    (FileType::File, pathname)
                }
            };
            packages
                .entry(packagename.clone())
                .or_insert_with(|| CMakePackage {
                    name: packagename,
                    filetype: packagetype,
                    filepath: packagepath,
                    version,
                    tojump,
                    from: "Vcpkg".to_string(),
                });
        }
    }
    packages
}

pub fn make_vcpkg_package_search_path(search_path: &Path) -> std::io::Result<Vec<String>> {
    const LIB_PATHS: [&str; 5] = [
        "x64-linux",
        "x86-linux",
        "x64-windows",
        "x86-windows",
        "x64-osx",
    ];

    let mut paths: Vec<String> = Vec::new();

    // check search path is ok
    for item in LIB_PATHS {
        if search_path.join(item).is_dir() {
            let path = Path::new(item).join("share");
            paths.push(path.to_str().unwrap().to_string());
        }
    }

    Ok(paths)
}

pub static VCPKG_CMAKE_PACKAGES: LazyLock<Vec<CMakePackage>> =
    LazyLock::new(|| get_cmake_message().into_values().collect());
pub static VCPKG_CMAKE_PACKAGES_WITHKEY: LazyLock<HashMap<String, CMakePackage>> =
    LazyLock::new(get_cmake_message);
