use std::{
    fs::File,
    io::{self, BufWriter},
    path::{Path, PathBuf},
    process::{ExitStatus, Stdio},
};

use clap::Command;
use clap_complete::{generate, Shell};
use shellexpand::tilde;

use crate::gendata::CollisionResolver;


/// Shell Tool
pub fn runit<'a>(args: &str) -> Result<ExitStatus, io::Error> {
    std::process::Command::new("sh")
        .arg("-c")
        .arg(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .status()
}

#[macro_export]
macro_rules! run {
    ($s:literal, $($rem:tt)*) => {{
        let formated_s = format!($s, $($rem)*);

        run!(&formated_s)
    }};
    ($s:expr) => {{
        use $crate::runit;

        runit($s)
    }};
}

#[macro_export]
macro_rules! path {
    ($s:literal, $($rem:tt)*) => {{
        let formated_s = format!($s, $($rem)*);

        path!(&formated_s)
    }};
    ($s:expr) => {{
        use std::path::PathBuf;
        use std::str::FromStr;

        PathBuf::from_str($s).unwrap()
    }};
}


pub const BUNDLE_NAME: &'static str = "db.bin";
pub const BUNDLE_DUP_NAME: &'static str = "db_dup.bin";


#[inline]
pub fn tbl_dir(id: u32) -> PathBuf {
    path!("data{}", id)
}
#[inline]
pub fn tbl_path(id: u32) -> PathBuf {
    tbl_dir(id).join(BUNDLE_NAME)
}


#[inline]
pub fn tbl_dup_dir() -> PathBuf {
    path!("data_dup")
}
#[inline]
pub fn tbl_dup_path(id: u32) -> PathBuf {
    tbl_dup_dir().join(path!("db_dup_{}.bin", id))
}


#[inline]
pub fn path2str(p: &Path) -> String {
    p.as_os_str().to_string_lossy().to_string()
}

impl CollisionResolver {
    fn init(&self) {
        match self {
            CollisionResolver::Rehash => {
                run!("mkdir -p {}", "data_cr_rehash").unwrap();
            }
        }
    }

    fn pathbuf(&self, id: u32) -> PathBuf {
        match self {
            CollisionResolver::Rehash => path!("data_cr_rehash")
                .join(format!("db_cr_rehash_{}.bin", id)),
        }
    }
}



#[derive(Clone, Copy)]
pub(crate) enum TblTy {
    Normal(u32),
    Dup(u32),
    Resolve(u32, CollisionResolver),
}

impl TblTy {
    pub fn init(&self) {
        match self {
            Self::Normal(id) => {
                run!("mkdir -p {}", path2str(&tbl_dir(*id))).unwrap();
            }
            Self::Dup(_) => {
                run!("mkdir -p {}", path2str(&tbl_dup_dir())).unwrap();
            }
            Self::Resolve(_id, resolv) => resolv.init(),
        }
    }

    pub fn pathbuf(&self) -> PathBuf {
        match self {
            Self::Normal(id) => tbl_path(*id),
            Self::Dup(id) => tbl_dup_path(*id),
            Self::Resolve(id, resolv) => resolv.pathbuf(*id),
        }
    }

    pub fn add(&self) -> Self {
        match self {
            Self::Normal(id) => Self::Normal(*id + 1),
            Self::Dup(id) => Self::Dup(*id + 1),
            Self::Resolve(id, resolv) => {
                Self::Resolve(*id + 1, *resolv)
            }
        }
    }
}



pub fn gen_completions(gen: Shell, cmd: &mut Command) {
    match gen.to_string().to_uppercase().as_str() {
        "BASH" => {
            let t = tilde("~/.local/share/bash-completion/completions/");
            let dir = PathBuf::from(t.to_string());

            // let bin_name = "hhdm";
            let bin_name = cmd.get_bin_name().unwrap().to_string();
            let fullpath = dir.join(&bin_name);

            let f = File::create(fullpath).unwrap();
            let mut writer = BufWriter::new(f);

            // generate(gen, cmd, cmd.get_name().to_string(), &mut io::stdout());
            generate(gen, cmd, bin_name, &mut writer);
        }
        _ => unimplemented!(),
    }
}


#[cfg(test)]
mod tests {

    #[test]
    fn test_dec_macro() {
        use crate::run;

        run!("echo {} {} {}", 1, 2, 3).unwrap();
    }
}
