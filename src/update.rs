use colored::*;
use std::sync::{mpsc, Mutex};
use std::thread;

use crate::cache;
use crate::dpkg;
use crate::fetcher;
use crate::slist;
use crate::source;

pub fn do_update() {
  log::trace!("do_update()");

  let mut package_items = vec![];

  // read sources.list
  let sources = match slist::parseSourceFile("sources.list") {
    Ok(_items) => _items,
    Err(msg) => {
      println!("{}", msg);
      return;
    }
  };

  let start_time = std::time::SystemTime::now();
  // fetch index files and get package items.
  let mut handles = vec![];
  let (tx, rx) = mpsc::channel();
  println!("Fetching indexes... ");
  for ix in 0..sources.len() {
    let source = sources[ix].clone();
    let tx = tx.clone();
    let handle = thread::spawn(move || {
      println!("Get:{} {}", ix, source.info());
      let raw_index = match fetcher::fetchIndex(&source) {
        Ok(_raw_index) => _raw_index,
        Err(msg) => {
          println!("{}", msg);
          return;
        }
      };
      match cache::write_cache_raw(&raw_index, &source) {
        Ok(()) => {}
        Err(msg) => {
          println!("{}", msg);
          return;
        }
      }
      let fetched_size = raw_index.len() as u64;
      //println!(
      //  "{}:{} {} [{} B]",
      //  "Hit".blue(),
      //  ix,
      //  source.info(),
      //  raw_index.len()
      //);
      match source::SourcePackage::from_row(&raw_index) {
        Ok(mut _items) => {
          tx.send(Ok((fetched_size, _items))).unwrap();
        }
        Err(msg) => {
          tx.send(Err(msg)).unwrap();
        }
      }
    });
    handles.push(handle);
  }
  let mut fetched_amount = 0;
  for handle in handles {
    match rx.recv().unwrap() {
      Ok((fetched_size, mut item)) => {
        package_items.append(&mut item);
        fetched_amount += fetched_size;
      }
      Err(msg) => {
        println!("{}", msg);
        return;
      }
    }
    handle.join().unwrap();
  }
  let total_time = start_time.elapsed().unwrap().as_secs();
  let fetched_amount_kb: u64 = (fetched_amount / 1024).into();
  let bps = if total_time == 0 {
    fetched_amount_kb
  } else {
    fetched_amount_kb / total_time
  };
  println!(
    "Fetched {} kB in {}s ({} kB/s)",
    fetched_amount_kb, total_time, bps
  );

  print!("Reading package lists... ");
  let resolved_items = match source::resolve_duplication(&package_items) {
    Ok(_resolved_items) => _resolved_items,
    Err(msg) => {
      println!("\n{}", msg);
      return;
    }
  };
  println!("DONE");

  print!("Reading state information... ");
  let upgradable_items = match dpkg::check_upgradable(&resolved_items) {
    Ok(_upgradable_items) => _upgradable_items,
    Err(msg) => {
      println!("\n{}", msg);
      return;
    }
  };
  println!("DONE");
  if upgradable_items.len() != 0 {
    println!(
      "{} packages are upgradable.",
      upgradable_items.len().to_string().red().bold()
    );
  } else {
    println!("{}", "All packages are up to date.".green().bold());
  }
}
