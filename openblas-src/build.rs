use std::{env, fs, path::*, process::Command};

fn feature_enabled(feature: &str) -> bool {
    env::var(format!("CARGO_FEATURE_{}", feature.to_uppercase())).is_ok()
}

fn target_os() -> String {
    env::var("CARGO_CFG_TARGET_OS").unwrap()
}

fn binary() -> String {
    env::var("CARGO_CFG_TARGET_POINTER_WIDTH").unwrap()
}

/// Add path where pacman (on msys2) install OpenBLAS
///
/// - `pacman -S mingw-w64-x86_64-openblas` will install
///   - `libopenbla.dll` into `/mingw64/bin`
///   - `libopenbla.a`   into `/mingw64/lib`
/// - But we have to specify them using `-L` in **Windows manner**
///   - msys2 `/` is `C:\msys64\` in Windows by default install
///   - It can be convert using `cygpath` command
fn windows_gnu_system() {
    let lib_path = String::from_utf8(
        Command::new("cygpath")
            .arg("-w")
            .arg(if feature_enabled("static") {
                "/mingw64/bin"
            } else {
                "/mingw64/lib"
            })
            .output()
            .expect("Failed to exec cygpath")
            .stdout,
    )
    .expect("cygpath output includes non UTF-8 string");
    println!("cargo:rustc-link-search={}", lib_path);
}

/// Use vcpkg for msvc "system" feature
fn windows_msvc_system() {
    if feature_enabled("static") {
        env::set_var("CARGO_CFG_TARGET_FEATURE", "crt-static");
    } else {
        env::set_var("VCPKGRS_DYNAMIC", "1");
    }
    #[cfg(target_env = "msvc")]
    vcpkg::find_package("openblas").unwrap();
    if !cfg!(target_env = "msvc") {
        unreachable!();
    }
}

/// homebrew says
///
/// > openblas is keg-only, which means it was not symlinked into /usr/local,
/// > because macOS provides BLAS in Accelerate.framework.
/// > For compilers to find openblas you may need to set:
///
/// ```text
/// export LDFLAGS="-L/usr/local/opt/openblas/lib"
/// export CPPFLAGS="-I/usr/local/opt/openblas/include"
/// ```
fn macos_system() {
    println!("cargo:rustc-link-search=/usr/local/opt/openblas/lib");
}

fn main() {
    /*
    {
        "CARGO_MAKEFLAGS": "--jobserver-fds=3,4 -j --jobserver-auth=3,4 -j",
        "CARGO_FEATURE_DEFAULT": "1",
        "CARGO_FEATURE_CBLAS": "1",
        "CARGO_FEATURE_LAPACKE": "1",
        "CARGO_MANIFEST_LINKS": "openblas",
        "CARGO_MANIFEST_DIR": "/Users/songww/Workspace/imop.io/openblas-src/openblas-src",
        "CARGO_CFG_TARGET_HAS_ATOMIC": "128,16,32,64,8,ptr",
        "CARGO_CFG_TARGET_VENDOR": "apple",
        "CARGO_CFG_TARGET_FAMILY": "unix",
        "CARGO_CFG_TARGET_FEATURE": "crypto,fp,neon",
        "CARGO_CFG_TARGET_OS": "ios",
        "CARGO_CFG_TARGET_POINTER_WIDTH": "64",
        "CARGO_CFG_TARGET_HAS_ATOMIC_LOAD_STORE": "128,16,32,64,8,ptr",
        "CARGO_CFG_TARGET_ARCH": "aarch64",
        "CARGO_CFG_UNIX": "",
        "CARGO_CFG_TARGET_ENV": "",
        "CARGO_CFG_TARGET_HAS_ATOMIC_EQUAL_ALIGNMENT": "128,16,32,64,8,ptr"
        "CARGO_CFG_TARGET_ENDIAN": "little",
        "CARGO_PKG_REPOSITORY": "https://github.com/blas-lapack-rs/openblas-src",
        "CARGO_PKG_LICENSE_FILE": "",
        "CARGO_PKG_VERSION_PATCH": "1",
        "CARGO_PKG_HOMEPAGE": "https://github.com/blas-lapack-rs/openblas-src",
        "CARGO_PKG_NAME": "openblas-src",
        "CARGO_PKG_DESCRIPTION": "The package provides a source of BLAS and LAPACK via OpenBLAS.",
        "CARGO_PKG_VERSION_MAJOR": "0",
        "CARGO_PKG_VERSION_MINOR": "10",
        "CARGO_PKG_VERSION_PRE": "",
        "CARGO_PKG_LICENSE": "Apache-2.0/MIT",
        "CARGO_PKG_VERSION": "0.10.1",
        "CARGO_PKG_AUTHORS": "Corey Richardson <corey@octayn.net>:Ethan Smith <ethan@ethanhs.me>:Ivan Ukhov <ivan.ukhov@gmail.com>:Jim Turner <git@turner.link>:Ken Elkabany <ken@elkabany.com>:Steve Harris <steveOfAR@gmail.com>:Toshiki Teramura <toshiki.teramura@gmail.com>",
        "CARGO_HOME": "/Users/songww/.cargo",
    }'
    */
    let link_kind = if feature_enabled("static") {
        "static"
    } else {
        "dylib"
    };

    let target_os: &str = &target_os();
    let target_env: &str = &std::env::var("CARGO_CFG_TARGET_ENV").unwrap();

    if feature_enabled("system") {
        match target_os {
            "windows" => {
                match target_env {
                    "gnu" => {
                        windows_gnu_system();
                    },
                    "msvc" => {
                        windows_msvc_system();
                    },
                    _ => {
                        panic!(
                            "Unsupported ABI for Windows: {}",
                            env::var("CARGO_CFG_TARGET_ENV").unwrap()
                        );
                    }
                };
            },
            "macos" => {
                macos_system();
            },
            "ios" => {
                println!(
                    "cargo:rustc-link-search={}",
                    std::env::var("OPENBLAS_PREBUILD_PATH").expect("feature `system` is enabled, but `OPENBLAS_PREBUILD_PATH` env not set.")
                );
            },
            "android" => {
                println!(
                    "cargo:rustc-link-search={}",
                    std::env::var("OPENBLAS_PREBUILD_PATH").expect("feature `system` is enabled, but `OPENBLAS_PREBUILD_PATH` env not set.")
                );
            }
            _ => {
                panic!("platform {} not supported yet.", target_os);
            }
        }
        println!("cargo:rustc-link-lib={}=openblas", link_kind);
    } else {
        if target_env == "msvc" {
            panic!(
                "Non-vcpkg builds are not supported on Windows. You must use the 'system' feature."
            )
        }

        let output = PathBuf::from(env::var("OUT_DIR").unwrap().replace(r"\", "/"));
        let mut make = Command::new("make");
        make.args(&["libs", "netlib", "shared"])
            .arg(format!("BINARY={}", binary()))
            .arg(format!(
                "{}_CBLAS=1",
                if feature_enabled("cblas") {
                    "YES"
                } else {
                    "NO"
                }
            ))
            .arg(format!(
                "{}_LAPACKE=1",
                if feature_enabled("lapacke") {
                    "YES"
                } else {
                    "NO"
                }
            ));
        match env::var("OPENBLAS_ARGS") {
            Ok(args) => {
                make.args(args.split_whitespace());
            }
            _ => (),
        };
        if let Ok(num_jobs) = env::var("NUM_JOBS") {
            make.arg(format!("-j{}", num_jobs));
        }
        let target = match env::var("OPENBLAS_TARGET") {
            Ok(target) => {
                make.arg(format!("TARGET={}", target));
                target
            }
            _ => {
                let target = env::var("TARGET").unwrap();
                if target.starts_with("aarch64") {
                    make.arg("TARGET=ARMV8")
                        .arg("HOSTCC=clang")
                        .arg("CC=clang");
                    if target_os == "ios" {
                    let sdkroot = env::var("OPENBLAS_IOS_SDKROOT").unwrap_or_else(|_|{
                        String::from_utf8(Command::new("xcrun")
                            .args(&["--sdk", "iphoneos", "--show-sdk-path"])
                            .output()
                            .expect("ios sdk not found.")
                            .stdout).unwrap()
                    });
                        make.arg(format!("CFLAGS=-isysroot {} -arch arm64", sdkroot))
                        .arg("NOFORTRAN=1");
                    } else if target_os == "android" {

                    }
                    target
                } else if target.starts_with("armv7") || target.starts_with("thumbv7") || target.starts_with("arm-"){
                    let sdkroot = env::var("OPENBLAS_IOS_SDKROOT").unwrap_or_else(|_|{
                        String::from_utf8(Command::new("xcrun")
                            .args(&["--sdk", "iphoneos", "--show-sdk-path"])
                            .output()
                            .expect("ios sdk not found.")
                            .stdout).unwrap()
                    });
                    make.arg("TARGET=ARMV7")
                        .arg("HOSTCC=clang")
                        .arg("CC=clang")
                        .arg(format!("CFLAGS=-isysroot {} -arch arm64", sdkroot))
                        .arg("NOFORTRAN=1");
                    target
                } else if target.starts_with("x86_64") {
                    make.arg("TARGET=x86_64");
                    target
                } else if target.starts_with("i686") {
                    make.arg("TARGET=x86");
                    target
                } else {
                    panic!("target {} is not supported yet.", target);
                }
            }
        };
        env::remove_var("TARGET");
        let source = if feature_enabled("cache") {
            PathBuf::from(format!("source_{}", target.to_lowercase()))
        } else {
            output.join(format!("source_{}", target.to_lowercase()))
        };

        if !source.exists() {
            let source_tmp = PathBuf::from(format!("{}_tmp", source.display()));
            if source_tmp.exists() {
                fs::remove_dir_all(&source_tmp).unwrap();
            }
            run(Command::new("cp").arg("-R").arg("source").arg(&source_tmp));
            fs::rename(&source_tmp, &source).unwrap();
        }
        for name in &vec!["CC", "FC", "HOSTCC"] {
            if let Ok(value) = env::var(format!("OPENBLAS_{}", name)) {
                make.arg(format!("{}={}", name, value));
            }
        }
        run(&mut make.current_dir(&source));
        run(Command::new("make")
            .arg("install")
            .arg(format!("DESTDIR={}", output.display()))
            .current_dir(&source));
        println!(
            "cargo:rustc-link-search={}",
            output.join("opt/OpenBLAS/lib").display(),
        );
    }
    println!("cargo:rustc-link-lib={}=openblas", link_kind);
}

fn run(command: &mut Command) {
    println!("Running: `{:?}`", command);
    match command.status() {
        Ok(status) => {
            if !status.success() {
                panic!("Failed: `{:?}` ({})", command, status);
            }
        }
        Err(error) => {
            panic!("Failed: `{:?}` ({})", command, error);
        }
    }
}
