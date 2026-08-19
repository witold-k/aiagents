// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use std::path::{ Path, PathBuf };
use std::process::Command;

#[derive(Default, Debug, Clone)]
pub struct Buildcommand {
    pub setup: Vec<String>,
    pub build: Vec<String>,
    pub lint:  Vec<String>,
    pub test:  Vec<String>,
}

impl Buildcommand {
    pub fn new(
        setup: Vec<String>,
        build: Vec<String>,
        lint:  Vec<String>,
        test:  Vec<String>,
    ) -> Self {
        Self { setup, build, lint, test }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Buildsystem {
    None,
    Cargo,
    Cmake,
    Just,
    Make,
    Maven,
    Meson,
}

pub const ALL_BUILDSYSTEMS: &[Buildsystem] = &[
    Buildsystem::None,
    Buildsystem::Cargo,
    Buildsystem::Cmake,
    Buildsystem::Just,
    Buildsystem::Make,
    Buildsystem::Maven,
    Buildsystem::Meson,
];

impl Buildsystem {

    pub fn as_str(&self) -> &'static str {
        match self {
            Buildsystem::None => "None",
            Buildsystem::Cargo => "Cargo",
            Buildsystem::Cmake => "Cmake",
            Buildsystem::Just  => "Just",
            Buildsystem::Maven => "Maven",
            Buildsystem::Make  => "Make",
            Buildsystem::Meson => "Meson",
        }
    }

    pub fn iter() -> impl Iterator<Item = Buildsystem> {
        ALL_BUILDSYSTEMS.iter().copied()
    }

    pub fn len() -> usize {
        ALL_BUILDSYSTEMS.len()
    }

    pub fn to_vec_str() -> Vec<&'static str> {
        ALL_BUILDSYSTEMS.iter().map(|t| t.as_str()).collect()
    }

    pub fn is_cargo(&self) -> bool {
        Buildsystem::Cargo == *self
    }

    pub fn is_none(&self) -> bool {
        Buildsystem::None == *self
    }

    pub fn get_default_builddir(&self) -> PathBuf {
        match self {
            Buildsystem::None => PathBuf::from("."),
            Buildsystem::Cargo => PathBuf::from("target"),
            Buildsystem::Cmake => PathBuf::from("build"),
            Buildsystem::Just  => PathBuf::from("target"),
            Buildsystem::Make  => PathBuf::from("build"),
            Buildsystem::Maven => PathBuf::from("target"),
            Buildsystem::Meson => PathBuf::from("build"),
        }
    }

    /// Match by substring, case‑insensitive.
    pub fn from_name(name: &str) -> Self {
        let n = name.to_lowercase();

        static MAP: &[(&str, Buildsystem)] = &[
            ("cargo", Buildsystem::Cargo),
            ("cmake", Buildsystem::Cmake),
            ("just",  Buildsystem::Just),
            ("make",  Buildsystem::Make),
            ("maven", Buildsystem::Maven),
            ("meson", Buildsystem::Meson),
        ];

        for (key, variant) in MAP {
            if n.contains(key) {
                return *variant;
            }
        }

        Buildsystem::None
    }

    /// Detect build system by presence of known files.
    pub fn from_dir(p: &Path) -> Self {
        if p.join("Cargo.toml").exists() {
            Buildsystem::Cargo
        } else if p.join("meson.build").exists() {
            Buildsystem::Meson
        } else if p.join("CMakeLists.txt").exists() {
            Buildsystem::Cmake
        } else if p.join("pom.xml").exists() {
            Buildsystem::Maven
        } else if p.join("Justfile").exists() {
            Buildsystem::Just
        } else if p.join("Makefile").exists() {
            Buildsystem::Make
        } else {
            Buildsystem::None
        }
    }

    pub fn from_current_dir() -> Self {
        match std::env::current_dir() {
            Ok(path) => Self::from_dir(&path),
            Err(_) => Buildsystem::None,
        }
    }

    pub fn from_versioned_project(path: &Path) -> Self {
        let proj = fsscanner::pathutils::from_versioned_project(path);
        Self::from_dir(&proj)
    }

    pub fn setupbuild(&self, projdir: &Path, targetdir: &Path) {
        let bc: Vec<String> = self.build_cmd(projdir, targetdir).setup;

        if bc.is_empty() {
            return;
        }

        let (cmd, args) = bc.split_first().unwrap();

        let _ = Command::new(cmd)
            .args(args)
            .current_dir(projdir)
            .status();
    }

    pub fn build_cmd(&self, projdir: &Path, targetdir: &Path) -> Buildcommand {
        let targetdir = if targetdir.is_relative() {
            if let Ok(cwd) = std::env::current_dir() {
                cwd.join(targetdir)
            } else {
                // fallback: behave exactly like the else-branch
                targetdir.to_path_buf()
            }
        } else {
            targetdir.to_path_buf()
        };
        let sdir = projdir.to_string_lossy().to_string();
        let tdir = targetdir.to_string_lossy().to_string();


        match self {
            Buildsystem::Cargo => {
                let t = ["--target-dir".into(), tdir.clone()];
                Buildcommand::new(
                    vec![],
                    vec!["cargo".into(), "build".into(), t[0].clone(), t[1].clone()],
                    vec!["cargo".into(), "clippy".into(), t[0].clone(), t[1].clone()],
                    vec!["cargo".into(), "test".into(), t[0].clone(), t[1].clone()],
                )
            }

            Buildsystem::Cmake => {
                Buildcommand::new(
                    vec!["cmake".into(), "-S".into(), sdir.clone(), "-B".into(), tdir.clone(), "-G".into(), "Ninja".into()],
                    vec!["cmake".into(), "--build".into(), tdir.clone()],
                    vec![],
                    vec!["cmake".into(), "--build".into(), tdir.clone(), "--target".into(), "test".into()],
                )
            }

            Buildsystem::Just => {
                Buildcommand::new(
                    vec![],
                    vec!["just".into(), "build".into()],
                    vec![],
                    vec![],
                )
            }

            Buildsystem::Meson => {
                Buildcommand::new(
                    vec!["meson".into(), "setup".into(), tdir.clone()],
                    vec!["meson".into(), "compile".into(), "-C".into(), tdir.clone()],
                    vec![],
                    vec!["meson".into(), "test".into(), "-C".into(), tdir.clone()],
                )
            }

            Buildsystem::Make => {
                Buildcommand::new(
                    vec![],
                    vec!["make".into(), "build".into()],
                    vec![],
                    vec![],
                )
            }

            Buildsystem::Maven => {
                Buildcommand::new(
                    vec![
                        "mvn".into(),
                        "clean".into(),
                        format!("-DoutputDirectory={tdir}"),
                    ],
                    vec![
                        "mvn".into(),
                        "package".into(),
                        format!("-DoutputDirectory={tdir}"),
                    ],
                    vec![],
                    vec![
                        "mvn".into(),
                        "test".into(),
                        format!("-DoutputDirectory={tdir}"),
                    ],
                )
            }

            Buildsystem::None => Buildcommand::default(),
        }
    }
}

