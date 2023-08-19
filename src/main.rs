use std::{
	collections::HashMap,
	ffi::OsStr,
	fs,
	path::{Path, PathBuf},
};

use clap::{Parser, Subcommand};
use serde::Deserialize;

use monokakido as mkt;

mod utils;
use utils::*;

#[derive(Parser)]
#[command(version)]
struct Args {
	#[command(subcommand)]
	cmd: Cmds,
}

#[derive(Subcommand)]
enum Cmds {
	scan_base { dir: String },
	scan_dict { dir: String },
	scan_contents { dir: String },
}

#[derive(Deserialize, Debug)]
struct MkdProduct {
	#[serde(rename = "DSProductTitle")]
	title: HashMap<String, String>,
	#[serde(rename = "DSProductContents")]
	contents: Vec<MkdContent>,
}

#[derive(Deserialize, Debug)]
struct MkdContent {
	#[serde(rename = "DSContentTitle")]
	title: HashMap<String, String>,
	#[serde(rename = "DSContentDirectory")]
	dir: String,
}

fn main() {
	let args = Args::parse();
	match args.cmd {
		Cmds::scan_base { dir } => {
			scan_base(&dir);
		}
		Cmds::scan_dict { dir } => {
			scan_dict(&dir)
		}
		Cmds::scan_contents { dir } => {
			scan_contents(&dir)
		}
	}
}

// base: the base dir for all dicts
// each sub-dir should have a "Contents" sub-dir
// for example:
//	mac: "/Library/Application Support/AppStoreContent/jp.monokakido.Dictionaries/Products/"
fn scan_base<P: AsRef<Path>>(dir: P) {
	for e in fs::read_dir(dir).unwrap() {
		let e = e.unwrap();
		let p = e.path();
		if !p.is_dir() {
			continue;
		}
		scan_dict(p);
	}
}

fn scan_dict<P: AsRef<Path>>(dir: P) {
	let mut p: PathBuf = dir.as_ref().into();
	let dir_name = p.file_name().unwrap().to_str().unwrap().to_string();
	// main JSON
	p.push("Contents");
	let json = find_file_with_ext(&p, OsStr::new("json"));
	if json.len() != 1 {
		println!(
			"{} JSON file in {}, which is unexpected",
			json.len(),
			p.as_os_str().to_str().unwrap()
		);
		return;
	}
	let json = json[0].to_str().unwrap();
	// println!("{}", json);
	let json: MkdProduct = serde_json::from_reader(fs::File::open(json).unwrap()).unwrap();
	println!(
		"{} [{}]",
		fmt_ml_str(&json.title),
		dir_name,
	);
	for c in json.contents {
		println!("\t{} [{}]", fmt_ml_str(&c.title), &c.dir);
		p.push(c.dir);
		scan_contents(&p);
		p.pop();
	}
}

// dir: the content directory of a single dict
// should be a sub-dir of the "Contents" dir mentioned above
// should contain sub-dirs like "key"
fn scan_contents<P: AsRef<Path>>(dir: P) {
	for d in fs::read_dir(dir).unwrap() {
		let d = d.unwrap();
		let dp = d.path();
		if !dp.is_dir() {
			continue;
		}
		let dn = d.file_name();
		let dn = dn.to_str().unwrap();
		// counters
		let mut c_idx = 0;
		let mut c_nidx = 0;
		let mut c_keys = 0;
		let mut c_not_file = 0;
		let mut c_no_ext = 0;
		let mut c_other = HashMap::<String, isize>::new();
		// counter helper
		let mut c_other_mod = |e: &str, m: isize| {
			*c_other.entry(e.to_string()).or_insert(0) += m;
		};
		for f in fs::read_dir(&dp).unwrap() {
			let f = f.unwrap();
			let fp = f.path();
			if !fp.is_file() {
				c_not_file += 1;
				continue;
			}
			let fname = f.file_name();
			let fname = fname.to_str().unwrap();
			let fext = match fp.extension() {
				Some(e) => e,
				None => {
					c_no_ext += 1;
					continue;
				}
			};
			match fext.to_str().unwrap() {
				"idx" => {
					let mp = fp.with_extension("map");
					if mp.exists() && mp.is_file() {
						println!("\t\t{}: {}|map", dn, fname);
						// prevent the corresponding map file from showing up in c_other
						c_other_mod("map", -1);
					} else {
						println!(
							"\t\t{}: {} without corresponding map file, unexpected",
							dn, fname
						)
					}
				}
				"nidx" => {
					println!("\t\t{}: {}", dn, fname);
				}
				"keystore" => {
					println!("\t\t{}: {}", dn, fname);
				}
				e => {
					c_other_mod(e, 1);
				}
			};
		}
		// collect others and print them in a single line
		let mut r = Vec::with_capacity(c_other.keys().len() + 2);
		for (e, c) in c_other.iter() {
			if *c > 0 {
				r.push(format!("{}: {}", e, c));
			}
		}
		if c_no_ext > 0 {
			r.push(format!("no ext: {}", c_no_ext));
		}
		if c_not_file > 0 {
			r.push(format!("not file: {}", c_not_file));
		}
		if r.len() > 0 {
			println!("\t\t{}: {}", dn, r.join(", "));
		}
	}
}
