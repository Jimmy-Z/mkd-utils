use std::{
	collections::{HashMap, HashSet},
	ffi::OsStr,
	fs,
	path::{Path, PathBuf}, fmt::Display,
};

pub fn fmt_ml_str(h: &HashMap<String, String>) -> String {
	h.values()
		.map(|e| e.as_str())
		.collect::<HashSet<_>>()
		.into_iter()
		.collect::<Vec<_>>()
		.join(" | ")
}

pub fn find_file_with_ext<P: AsRef<Path>>(path: P, ext: &OsStr) -> Vec<PathBuf> {
	fs::read_dir(path)
		.unwrap()
		.filter_map(|e| {
			let p = e.unwrap().path();
			if p.is_file() && Some(ext) == p.extension() {
				Some(p)
			} else {
				None
			}
		})
		.collect()
}

pub struct Stats(HashMap<String, isize>);

impl Stats{
	pub fn new() -> Self {
		Self(HashMap::new())
	}

	pub fn len(&self) -> usize {
		self.0.len()
	}

	pub fn add(&mut self, k: impl ToString, v: isize) {
		*self.0.entry(k.to_string()).or_insert(0) += v;
	}
}

impl Display for Stats {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let mut r = Vec::with_capacity(self.len());
		for (e, c) in self.0.iter() {
			if *c > 0 {
				r.push(format!("{}: {}", e, c));
			}
		}
		write!(f, "{}", r.join(", "))
	}
}
