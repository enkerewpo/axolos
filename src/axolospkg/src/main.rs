use std::{
    collections::{BTreeMap, HashMap},
    io::BufRead,
    process::{Command, Stdio},
};

use clap::Parser;
use log::*;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
/// A tool to create a rootfs from a package
struct Args {
    /// Path to the package info list
    #[arg(short, long)]
    input_pkg_path: String,
    /// Path to the rootfs to install the package to
    #[arg(short, long)]
    output_path: String,
    /// path to the packages folder
    #[arg(short, long)]
    packages_path: String,
}

#[derive(Debug, Clone)]
struct PkginfoEntry {
    pub key: String,
    pub value: String,
}

impl PkginfoEntry {
    pub fn new(key: String, value: String) -> Self {
        Self { key, value }
    }
    pub fn has_placeholder(&self) -> bool {
        self.value.contains("{{")
    }
    pub fn get_unresolved_placeholder(&self) -> Vec<String> {
        let mut res = Vec::new();
        let idx = self.value.find("{{");
        if idx.is_none() {
            return res;
        }
        let mut tmp_value = self.value.clone();
        while let Some(idx) = tmp_value.find("{{") {
            let idx = idx + 2;
            let end_idx = tmp_value[idx..].find("}}");
            if end_idx.is_none() {
                break;
            }
            let end_idx = end_idx.unwrap() + idx;
            let placeholder = tmp_value[idx..end_idx].to_string();
            res.push(placeholder);
            tmp_value = tmp_value[end_idx..].to_string();
        }
        res
    }
}

fn main() {
    let args = Args::parse();
    // use simple logger
    simple_logger::init().unwrap();
    info!("welcome to axolospkg, a tool to auto build packages for axolos");
    let input_pkg_path = args.input_pkg_path;
    let output_path = args.output_path;
    let packages_path = args.packages_path;
    let input_pkg_path = std::fs::canonicalize(input_pkg_path).unwrap();
    let output_path = std::fs::canonicalize(output_path).unwrap();
    let packages_path = std::fs::canonicalize(packages_path).unwrap();
    info!("input_pkg_path: {:?}", input_pkg_path);
    info!("output_path: {:?}", output_path);
    info!("packages_path: {:?}", packages_path);
    // first we should read each line of the package info list
    // which indicates the pkg to be built
    let file = std::fs::File::open(input_pkg_path).unwrap();
    let reader = std::io::BufReader::new(file);
    // for each line, we should build the package, use read_line to read each line
    let mut pkgs = Vec::new();
    for line in reader.lines() {
        let line = line.unwrap();
        pkgs.push(line);
    }
    info!("pkgs: {:?}", pkgs);
    let output_path = output_path.join("pkgs");
    std::fs::create_dir_all(&output_path).unwrap();
    // since we got the packages folder, if we choose the package A
    // then its pkg config will be stored in packages/A/pkginfo
    for pkg in pkgs {
        let pkg_path = packages_path.join(pkg);
        let pkg_path = pkg_path.join("pkginfo");
        let pkg_path = std::fs::canonicalize(pkg_path).unwrap();
        info!("pkg_path: {:?}", pkg_path);
        // read the pkginfo file
        let file = std::fs::File::open(pkg_path).unwrap();
        let reader = std::io::BufReader::new(file);
        let mut pkg_info = BTreeMap::new();
        for line in reader.lines() {
            let line = line.unwrap();
            let mut parts = line.split("::=");
            let key = parts.next().unwrap().trim().to_string();
            let value = parts.next().unwrap().trim().to_string();
            let entry = PkginfoEntry::new(key.clone(), value);
            pkg_info.insert(key.clone(), entry);
        }
        // each line is like ket: value, and we can have something like this
        // a: b
        // c: {{a}}bb
        // d: {{c}}
        // the placeholder should always refer to key before it, not after it, so we
        // can resolve the placeholder in one pass
        let mut resolved = BTreeMap::new();
        for (key, entry) in pkg_info.iter() {
            if entry.has_placeholder() {
                let mut value = entry.value.clone();
                let placeholders = entry.get_unresolved_placeholder();
                for placeholder in placeholders {
                    info!("placeholder: {:?} in {:?}", placeholder, key);
                    let placeholder_entry = pkg_info.get(&placeholder).unwrap();
                    let placeholder_value = placeholder_entry.value.clone();
                    value = value.replace(&format!("{{{{{}}}}}", placeholder), &placeholder_value);
                }
                resolved.insert(key.clone(), PkginfoEntry::new(key.clone(), value));
            } else {
                resolved.insert(key.clone(), entry.clone());
            }
        }
        // now let's create a folder for the package
        let pkg_output_path = output_path.join(resolved.get("PACKAGE_NAME").unwrap().value.clone());
        std::fs::create_dir_all(&pkg_output_path).unwrap();
        let src_dl_url = resolved.get("PACKAGE_SRC").unwrap().value.clone();
        let download_filename = resolved.get("PACKAGE_DL_FILENAME").unwrap().value.clone();
        let downlaod_file_full_path = pkg_output_path.join(download_filename.clone());
        info!("downlaod_file_full_path: {:?}", downlaod_file_full_path);
        if downlaod_file_full_path.exists() {
            info!("package already downloaded, skipping download");
        } else {
            // download the package into the folder use wget, print the realtime output
            let child = Command::new("wget")
                .arg("-P")
                .arg(&pkg_output_path)
                .arg(&src_dl_url)
                .spawn()
                .unwrap();
            child.wait_with_output().expect("failed to wait on child");
            // TODO: print the realtime output of command
            // TODO: get the downlaoded file name
            // info!("wget output: {:?}", output);
            info!(
                "downloaded pkg to {:?}",
                pkg_output_path.join(download_filename.clone())
            );
            // now we should extract the package
            // TODO: select unzip or tar based on the file extension
            // first check if ends with .zip
            if download_filename.ends_with(".zip") {
                let child = Command::new("unzip")
                    .arg(&pkg_output_path.join(download_filename.clone()))
                    .arg("-d")
                    .arg(&pkg_output_path)
                    .spawn()
                    .unwrap();
                child.wait_with_output().expect("failed to wait on child");
                info!("extracted pkg {} to {:?}", src_dl_url, pkg_output_path);
            } else if download_filename.ends_with(".tar.gz") {
                let child = Command::new("tar")
                    .arg("-xf")
                    .arg(&pkg_output_path.join(download_filename.clone()))
                    .arg("-C")
                    .arg(&pkg_output_path)
                    .spawn()
                    .unwrap();
                child.wait_with_output().expect("failed to wait on child");
                info!("extracted pkg {} to {:?}", src_dl_url, pkg_output_path);
            } else if download_filename.ends_with(".tar.xz") {
                let child = Command::new("tar")
                    .arg("-xf")
                    .arg(&pkg_output_path.join(download_filename.clone()))
                    .arg("-C")
                    .arg(&pkg_output_path)
                    .spawn()
                    .unwrap();
                child.wait_with_output().expect("failed to wait on child");
                info!("extracted pkg {} to {:?}", src_dl_url, pkg_output_path);
            }
        }
        // TODO: build the package according to the pkginfo's BUILD_COMMAND
        let build_root = resolved.get("PACKAGE_BUILD_ROOT").unwrap().value.clone();
        let real_build_dir = pkg_output_path.join(build_root);
        info!("real_build_dir: {:?}", real_build_dir);
        let target_program = resolved.get("TARGET_PROG").unwrap().value.clone();
        let target_program = real_build_dir.join(target_program);
        info!("target_program: {:?}", target_program);
        if target_program.exists() {
            info!("target program already built, skipping build");
        } else {
            let build_cmd = resolved.get("PACKAGE_BUILD_CMD").unwrap().value.clone();
            info!("build_cmd: {:?}", build_cmd);
            let child = Command::new("sh")
                .arg("-c")
                .arg(build_cmd)
                .current_dir(&real_build_dir)
                .spawn()
                .expect("failed to execute process");
            child.wait_with_output().expect("failed to wait on child");
        }
        // copy the target program to the output path's rootfs
        let target_program_output_path = output_path.join("../rootfs");
        let target_program_output_path =
            target_program_output_path.join(target_program.file_name().unwrap());
        info!(
            "target_program_output_path: {:?}",
            target_program_output_path
        );
        std::fs::copy(&target_program, &target_program_output_path).expect("failed to copy");
        info!("copied target program to {:?}", target_program_output_path);
        info!("build finished");
    }
    info!("done");
}
