use std::{
	collections::{HashMap, HashSet},
	ffi::OsStr,
	fs,
	path::{Path, PathBuf},
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
