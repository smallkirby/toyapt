use crate::slist;
use crate::source;
use crate::source::SourcePackage;
use glob::{glob, Pattern};
use std::fs;
use std::io::{Error, Write};
use std::path::Path;

pub fn search_cache_with_name(glob: &Pattern) -> Vec<SourcePackage> {
  let mut ret_items = vec![];
  let cached_items = get_cached_items();
  for item in cached_items {
    if glob.matches(&item.package) {
      ret_items.push(item.clone());
    }
  }

  ret_items
}

pub fn get_cached_items() -> Vec<SourcePackage> {
  let mut ret_items = vec![];

  match glob::glob("lists/*") {
    Ok(paths) => {
      for entry in paths {
        match entry {
          Ok(path) => {
            let raw_cache = match std::fs::read_to_string(path) {
              Ok(_raw_cache) => _raw_cache,
              Err(msg) => {
                println!("{}", msg);
                return vec![];
              }
            };
            match source::SourcePackage::from_row(&raw_cache) {
              Ok(mut _items) => ret_items.append(&mut _items),
              Err(msg) => {
                println!("{}", msg);
                return ret_items;
              }
            };
          }
          Err(msg) => {
            println!("failed to open cache file.");
            return vec![];
          }
        };
      }
    }
    Err(_) => {
      println!("invalid glob pattern.");
      return vec![];
    }
  };

  ret_items
}

pub fn write_cache_raw(raw_index: &str, source: &slist::Source) -> Result<(), String> {
  let filename = source.to_filename();
  if !Path::new("lists").exists() {
    return Err("cache directory 'lists' doesn't exist. aborting...".to_string());
  };
  if Path::new(&format!("lists/{}", filename)).exists() {
    // clean the file for simplicity
    fs::remove_file(format!("lists/{}", filename)).unwrap();
  };

  log::info!("creating cache file: {}", format!("lists/{}", filename));
  let mut out = fs::File::create(format!("lists/{}", filename)).unwrap();
  write!(out, "{}", raw_index).unwrap();

  Ok(())
}