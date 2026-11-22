// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

//! aiagents-specific build-system adapter.
//!
//! Command profiles and process execution live in the external build-wrapper
//! crate. This module retains aiagents' language inference and legacy enum
//! names for callers that select workflows.

use crate::generated_languages::Languages;
use crate::runtimetools::buildresult::Buildresult;
use crate::runtimetools::generic_work_step::run_cmd;
use build_wrapper::{BuildCommands, BuildSystem};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Buildsystem {
    None,
    Cargo,
    Cmake,
    Just,
    Lake,
    Make,
    Maven,
    Meson,
}

pub const ALL_BUILDSYSTEMS: &[Buildsystem] = &[
    Buildsystem::None,
    Buildsystem::Cargo,
    Buildsystem::Cmake,
    Buildsystem::Just,
    Buildsystem::Lake,
    Buildsystem::Make,
    Buildsystem::Maven,
    Buildsystem::Meson,
];

impl Buildsystem {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Cargo => "Cargo",
            Self::Cmake => "Cmake",
            Self::Just => "Just",
            Self::Lake => "Lake",
            Self::Make => "Make",
            Self::Maven => "Maven",
            Self::Meson => "Meson",
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

    pub fn inferred_language(&self) -> Option<Languages> {
        match self {
            Self::Cargo => Some(Languages::Rust),
            Self::Cmake | Self::Meson => Some(Languages::Cpp),
            Self::Maven => Some(Languages::Java),
            Self::None | Self::Just | Self::Lake | Self::Make => None,
        }
    }

    pub fn is_cargo(&self) -> bool {
        *self == Self::Cargo
    }

    pub fn is_none(&self) -> bool {
        *self == Self::None
    }

    pub fn get_default_builddir(&self) -> PathBuf {
        match self {
            Self::None => PathBuf::from("."),
            Self::Cargo | Self::Just | Self::Maven => PathBuf::from("target"),
            Self::Lake => PathBuf::from(".lake/build"),
            Self::Cmake | Self::Make | Self::Meson => PathBuf::from("build"),
        }
    }

    /// Compatibility for names supplied by the existing CLI.
    pub fn from_name(name: &str) -> Self {
        let name = name.to_ascii_lowercase();
        for (key, system) in [
            ("cargo", Self::Cargo),
            ("cmake", Self::Cmake),
            ("just", Self::Just),
            ("lake", Self::Lake),
            ("lean", Self::Lake),
            ("make", Self::Make),
            ("maven", Self::Maven),
            ("meson", Self::Meson),
        ] {
            if name.contains(key) {
                return system;
            }
        }
        Self::None
    }

    pub fn from_dir(dir: &Path) -> Self {
        BuildSystem::detect(dir)
            .map(Self::from)
            .unwrap_or(Self::None)
    }

    pub fn from_current_dir() -> Self {
        std::env::current_dir().map_or(Self::None, |dir| Self::from_dir(&dir))
    }

    pub fn from_versioned_project(path: &Path) -> Self {
        let root = fsscanner::pathutils::from_versioned_project(path);
        Self::from_dir(&root)
    }

    fn to_external(self) -> Option<BuildSystem> {
        match self {
            Self::None => None,
            Self::Cargo => Some(BuildSystem::Cargo),
            Self::Cmake => Some(BuildSystem::Cmake),
            Self::Just => Some(BuildSystem::Just),
            Self::Lake => Some(BuildSystem::Lake),
            Self::Make => Some(BuildSystem::Make),
            Self::Maven => Some(BuildSystem::Maven),
            Self::Meson => Some(BuildSystem::Meson),
        }
    }

    /// Delegate all command construction to the standalone BLT library.
    pub fn build_cmd(&self, projdir: &Path, targetdir: &Path) -> BuildCommands {
        self.to_external()
            .map(|system| system.commands(projdir, targetdir))
            .unwrap_or_default()
    }

    /// Legacy setup task, now reporting failures rather than silently ignoring them.
    pub fn setupbuild(&self, projdir: &Path, targetdir: &Path) -> Buildresult {
        let commands = self.build_cmd(projdir, targetdir);
        run_cmd(projdir, &commands.setup)
    }
}

impl From<BuildSystem> for Buildsystem {
    fn from(system: BuildSystem) -> Self {
        match system {
            BuildSystem::Cargo => Self::Cargo,
            BuildSystem::Cmake => Self::Cmake,
            BuildSystem::Just => Self::Just,
            BuildSystem::Lake => Self::Lake,
            BuildSystem::Make => Self::Make,
            BuildSystem::Maven => Self::Maven,
            BuildSystem::Meson => Self::Meson,
        }
    }
}
