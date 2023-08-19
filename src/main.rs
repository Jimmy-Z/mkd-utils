use std::{
	collections::HashMap,
	ffi::OsStr,
	fs,
	io::Write,
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

	/// explode items from resource archives, dry-run if not specified (default)
	#[arg(long, short)]
	explode: bool,

	/// do not enumerate items in resource archives, has no effect when exploding
	#[arg(long, short)]
	shallow: bool,

	#[arg(long, short, default_value = "out")]
	out_dir: String,
}

#[derive(Subcommand)]
enum Cmds {
	/// scan a base dir, contains dictionary sub dirs (more description below)
	ScanBase { dir: String },

	/// scan a dictionary dir, should contain a "Contents" sub dir
	ScanDict { dir: String },

	/// scan a content dir, should be a sub dir of the "Contents" dir mentioned above,
	///	should contain sub dirs like "contents", "key", "audio"
	ScanContents { dir: String },
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
		Cmds::ScanBase { dir } => {
			scan_base(&dir, args.explode, args.shallow, &args.out_dir);
		}
		Cmds::ScanDict { dir } => scan_dict(&dir, args.explode, args.shallow, &args.out_dir),
		Cmds::ScanContents { dir } => {
			scan_contents(&dir, args.explode, args.shallow, &args.out_dir)
		}
	}
}

fn scan_base<D: AsRef<Path>, O: AsRef<Path>>(dir: D, explode: bool, shallow: bool, out_dir: O) {
	for e in fs::read_dir(dir).unwrap() {
		let e = e.unwrap();
		let p = e.path();
		if !p.is_dir() {
			continue;
		}
		scan_dict(p, explode, shallow, &out_dir);
	}
}

fn scan_dict<D: AsRef<Path>, O: AsRef<Path>>(dir: D, explode: bool, shallow: bool, out_dir: O) {
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
	println!("{} [{}]", fmt_ml_str(&json.title), dir_name,);
	let mut out_dir = out_dir.as_ref().to_path_buf();
	for c in json.contents {
		println!("\t{} [{}]", fmt_ml_str(&c.title), &c.dir);
		p.push(&c.dir);
		out_dir.push(&c.dir);
		scan_contents(&p, explode, shallow, &out_dir);
		p.pop();
		out_dir.pop();
	}
}

fn scan_contents<D: AsRef<Path>, O: AsRef<Path>>(dir: D, explode: bool, shallow: bool, out_dir: O) {
	for d in fs::read_dir(dir).unwrap() {
		let d = d.unwrap();
		let dp = d.path();
		if !dp.is_dir() {
			continue;
		}
		let dn = d.file_name();
		let dn = dn.to_str().unwrap();

		// lists and counters
		let mut l_toc = HashMap::<String, Vec<String>>::new();
		let mut c_not_file = 0;
		let mut c_no_ext = 0;
		let mut c_other = HashMap::<String, isize>::new();
		// counter helper
		let mut l_toc_add = |e: &str, n: &str| {
			l_toc
				.entry(e.to_string())
				.or_insert(Vec::new())
				.push(n.to_string());
		};
		let mut c_other_mod = |e: &str, m: isize| {
			*c_other.entry(e.to_string()).or_insert(0) += m;
		};
		let mut out_dir = out_dir.as_ref().to_path_buf();
		out_dir.push(dn);
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
			let fext = fext.to_str().unwrap();
			match fext {
				"map" | "idx" | "nidx" | "keystore" => {
					println!("\t\t{}: {}", dn, fname);
					l_toc_add(fext, fname);
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

		let safe_get = |k: &str| match l_toc.get(k) {
			Some(v) => &v[..],
			None => &[],
		};
		if safe_get("map").len() == 1 {
			match mkt::resource::Rsc::new(&dp, &dn) {
				Ok(mut rsc) => {
					println!("\t\t\tinitilized rsc(map|idx)");
					rsc.explode(explode, shallow, &out_dir, derive_ext(dn));
				}
				Err(e) => {
					eprintln!("failed to parse rsc(map|idx): {:?}", e);
				}
			}
		}
		if safe_get("nidx").len() == 1 {
			match mkt::resource::Nrsc::new(&dp) {
				Ok(mut nrsc) => {
					println!("\t\t\tinitilized nrsc(nidx)");
					nrsc.explode(explode, shallow, &out_dir, derive_ext(dn));
				}
				Err(e) => {
					eprintln!("failed to parse nrsc(nidx): {:?}", e);
				}
			}
		}
		for k in safe_get("keystore").iter() {
			let mut p = dp.clone();
			p.push(k);
			match mkt::Keys::new(p) {
				Ok(keys) => {
					println!("\t\t\tinitilized keystore from {}", k);
					// TODO
					// keys.explode(explode, out_dir);
				}
				Err(e) => {
					eprintln!("failed to parse keystore from {}: {:?}", k, e);
				}
			}
		}
	}
}

trait Explode {
	fn len(&self) -> usize;
	fn get(&mut self, idx: usize) -> Result<(String, &[u8]), mkt::Error>;

	fn explode<P: AsRef<Path>>(
		&mut self,
		do_explode: bool,
		shallow: bool,
		dir: P,
		ext: Option<&str>,
	) {
		println!("\t\t\t\t{} entires", self.len());
		if !do_explode && shallow {
			return;
		}

		if do_explode {
			if let Err(e) = fs::create_dir_all(&dir) {
				eprintln!(
					"failed to create dir {}: {}",
					dir.as_ref().as_os_str().to_str().unwrap(),
					e
				);
				return;
			}
		}

		let mut p = dir.as_ref().to_path_buf();
		for idx in 0..self.len() {
			let (id, asset) = match self.get(idx) {
				Ok(r) => r,
				Err(e) => {
					eprintln!("failed to get resource {}: {:?}", idx, e);
					continue;
				}
			};

			let an = match ext {
				Some(ext) => format!("{}.{}", id, ext),
				None => id,
			};
			println!("\t\t\t\t{}", &an);
			if !do_explode {
				continue;
			}
			p.push(an);
			match fs::File::create(&p) {
				Ok(mut f) => match f.write_all(asset) {
					Ok(()) => {}
					Err(e) => {
						eprintln!(
							"error writing file {}: {}",
							&p.as_os_str().to_str().unwrap(),
							e
						);
					}
				},
				Err(e) => {
					eprintln!(
						"failed to create file {}: {}",
						&p.as_os_str().to_str().unwrap(),
						e
					);
				}
			}
			p.pop();
		}
	}
}

impl Explode for mkt::resource::Rsc {
	fn len(&self) -> usize {
		self.len()
	}

	fn get(&mut self, idx: usize) -> Result<(String, &[u8]), mkt::Error> {
		let (id, asset) = self.get_by_idx(idx)?;
		Ok((format!("{:0>10}", id), asset))
	}
}

impl Explode for mkt::resource::Nrsc {
	fn len(&self) -> usize {
		self.len()
	}

	fn get(&mut self, idx: usize) -> Result<(String, &[u8]), mkt::Error> {
		let (id, asset) = self.get_by_idx(idx)?;
		Ok((id.to_string(), asset))
	}
}

fn derive_ext(c_name: &str) -> Option<&'static str> {
	match c_name {
		"audio" => Some("aac"),
		"graphics" => None,
		"contents" => Some("xml"),
		_ => Some("bin"),
	}
}
