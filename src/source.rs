use crate::cache;
use crate::version::*;
use indicatif::ProgressBar;
use std::{cmp::Ordering, collections::HashMap};
use strum_macros::Display;

#[derive(Debug, PartialEq, Default, Clone)]
pub struct SourcePackage {
  pub package: String,
  pub binary: Vec<String>,
  pub arch: Vec<Arch>,
  pub version: String,
  pub priority: Priority,
  pub section: Section,
  pub maintainer: String,
  pub original_maintainer: String,
  pub uploaders: Vec<String>,
  pub standard_version: String,
  pub depends: HashMap<String, Option<String>>,
  pub pre_depends: HashMap<String, Option<String>>,
  pub testsuite: String,
  pub homepage: String,
  pub directory: String,
  pub chksum_md5: String,
  pub essential: bool,
  pub suggests: Vec<String>,
  pub filename: String,
  pub description: String,
  pub conffiles: Vec<String>,
  pub origin: String,
  pub bugs: String,
}

impl SourcePackage {
  pub fn to_pool_uri(&self) -> Result<String, ()> {
    let mut puri = String::new();
    puri.push_str("http");
    puri.push_str("://");
    puri.push_str("");
    match cache::get_pool_domain(self) {
      Ok(domain) => puri.push_str(&domain),
      Err(()) => return Err(()),
    };
    puri.push_str(&self.filename);

    Ok(puri)
  }

  pub fn from_row(file: &str) -> Result<Vec<Self>, String> {
    let mut items = vec![];
    let mut item = SourcePackage::default();
    let lines = file.split("\n").collect::<Vec<_>>();
    let mut cont_description = false;
    let mut cont_conffiles = false;
    let mut tmp_description = String::new();

    for (ix, line) in lines.iter().enumerate() {
      if cont_description {
        if ix < lines.len() - 1
          && lines.iter().nth(ix + 1).unwrap().len() >= 1
          && lines
            .iter()
            .nth(ix + 1)
            .unwrap()
            .chars()
            .into_iter()
            .nth(0)
            .unwrap()
            == ' '
        {
          // multi-line continues
          if cont_description {
            cont_description = true;
            tmp_description.push_str(&format!("\n{}", &line[1..]));
          } else if cont_conffiles {
            cont_conffiles = true;
            item.conffiles.push(format!("\n{}", &line[1..]));
          } else {
            panic!("unknown error while processing multi-line.");
          }
        } else {
          // multi-line ends here
          if cont_description {
            cont_description = false;
            tmp_description.push_str(&format!("\n{}", &line[1..]));
            item.description = tmp_description;
            tmp_description = String::new();
          } else if cont_conffiles {
            cont_conffiles = false;
            item.conffiles.push(format!("\n{}", &line[1..]));
          } else {
            panic!("unknown error while processing multi-line.");
          }
        }
        continue;
      }

      if line.len() == 0 {
        match item.verify() {
          Ok(()) => {
            items.push(item.clone());
            item = SourcePackage::default();
            continue;
          }
          Err(msg) => return Err(msg),
        }
      }
      let _parts = line.split(": ").collect::<Vec<_>>();
      let mut parts = _parts.iter();
      let title = parts.nth(0).unwrap();
      match *title {
        "Package" => {
          item.package = parts
            .nth(0)
            .ok_or(format!("invalid 'Package' format: {}", line))?
            .to_string();
        }
        "Architecture" => {
          let arch = parts
            .nth(0)
            .ok_or(format!("invalid 'Architecture' format: {}", line))?;
          item.arch.push(match *arch {
            "amd64" => Arch::AMD64,
            "all" => Arch::ALL,
            "any" => Arch::ANY,
            _ => Arch::UNKNOWN,
          });
        }
        "Version" => {
          item.version = parts
            .nth(0)
            .ok_or(format!("invalid 'Version' format: {}", line))?
            .to_string();
        }
        "Priority" => {
          item.priority = match *parts
            .nth(0)
            .ok_or(format!("invalid 'Priority' format: {}", line))?
          {
            "extra" => Priority::EXTRA,
            "optional" => Priority::OPTIONAL,
            "important" => Priority::IMPORTANT,
            "required" => Priority::REQUIRED,
            "standard" => Priority::STANDARD,
            _ => Priority::UNKNOWN,
          };
        }
        "Essential" => {
          item.essential = match *parts
            .nth(0)
            .ok_or(format!("invalid 'Essential' format: {}", line))?
          {
            "yes" => true,
            _ => false,
          };
        }
        "Section" => {
          item.section = match *parts
            .nth(0)
            .ok_or(format!("invalid 'Section' format: {}", line))?
          {
            "admin" => Section::ADMIN,
            _ => Section::UNKNOWN, // XXX
          };
        }
        "Maintainer" => {
          item.maintainer = parts
            .map(|s| String::from(*s))
            .collect::<Vec<_>>()
            .join(" ");
        }
        "Original-Maintainer" => {
          item.original_maintainer = parts
            .map(|s| String::from(*s))
            .collect::<Vec<_>>()
            .join(" ");
        }
        "Pre-Depends" => {
          let _depends = parts.map(|s| String::from(*s)).collect::<Vec<_>>().join("");
          let depends = _depends.split(",").collect::<Vec<_>>();
          for dep in depends {
            match parse_depends(dep) {
              Ok((pkg, version)) => item.pre_depends.insert(pkg, version),
              Err(msg) => return Err(msg),
            };
          }
        }
        "Depends" => {
          let _depends = parts.map(|s| String::from(*s)).collect::<Vec<_>>().join("");
          let depends = _depends.split(",").collect::<Vec<_>>();
          for dep in depends {
            match parse_depends(dep) {
              Ok((pkg, version)) => item.depends.insert(pkg, version),
              Err(msg) => return Err(msg),
            };
          }
        }
        "Suggests" => {
          let _sug = parts.map(|s| String::from(*s)).collect::<Vec<_>>().join("");
          let sug = _sug.split(",").map(|s| s.trim()).collect::<Vec<_>>();
          for s in sug {
            item.suggests.push(s.to_string());
          }
        }
        "Breaks" => {
          //log::debug!("ignoring Breaks.");
        }
        "Filename" => {
          item.filename = parts
            .nth(0)
            .ok_or(format!("invalid 'Filename' format: {}", line))?
            .to_string();
        }
        "MD5sum" => {
          item.chksum_md5 = parts
            .nth(0)
            .ok_or(format!("invalid 'MD5sum' format: {}", line))?
            .to_string();
        }
        "Homepage" => {
          item.homepage = parts
            .nth(0)
            .ok_or(format!("invalid 'Homepage' format: {}", line))?
            .to_string();
        }
        "Description" => {
          if ix < lines.len() - 1
            && lines.iter().nth(ix + 1).unwrap().len() >= 1
            && lines
              .iter()
              .nth(ix + 1)
              .unwrap()
              .chars()
              .into_iter()
              .nth(0)
              .unwrap()
              == ' '
          {
            cont_description = true;
            tmp_description.push_str(
              &parts
                .map(|s| String::from(*s))
                .collect::<Vec<_>>()
                .join(" "),
            );
          } else {
            cont_description = false;
            tmp_description.push_str(
              &parts
                .map(|s| String::from(*s))
                .collect::<Vec<_>>()
                .join(" "),
            );
            item.description = tmp_description;
            tmp_description = String::new();
          }
        }
        "Conffiles" => {
          if ix < lines.len() - 1
            && lines.iter().nth(ix + 1).unwrap().len() >= 1
            && lines
              .iter()
              .nth(ix + 1)
              .unwrap()
              .chars()
              .into_iter()
              .nth(0)
              .unwrap()
              == ' '
          {
            cont_conffiles = true;
          } else {
            cont_conffiles = false;
          }
          item.conffiles.push(
            parts
              .map(|s| String::from(*s))
              .collect::<Vec<_>>()
              .join(" "),
          );
        }
        "Origin" => {
          item.origin = parts
            .nth(0)
            .ok_or(format!("invalid 'Origin' format: {}", line))?
            .to_string();
        }
        "Bugs" => {
          item.bugs = parts
            .nth(0)
            .ok_or(format!("invalid 'Bugs' format: {}", line))?
            .to_string();
        }
        _ => {
          //log::debug!(
          //  "{}: ignoring unknown package field: {}",
          //  item.package,
          //  title
          //);
        }
      }
    }

    Ok(items)
  }

  pub fn verify(&self) -> Result<(), String> {
    Ok(())
  }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Arch {
  ALL,
  ANY,
  AMD64,
  UNKNOWN,
}

impl std::fmt::Display for Arch {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::ALL => write!(f, "all"),
      Self::ANY => write!(f, "any"),
      Self::AMD64 => write!(f, "amd64"),
      Self::UNKNOWN => write!(f, "unknown"),
    }
  }
}

#[derive(Debug, PartialEq, Copy, Clone, Display)]
pub enum Priority {
  REQUIRED,
  IMPORTANT,
  STANDARD,
  OPTIONAL,
  EXTRA,
  UNKNOWN,
}

impl PartialOrd for Priority {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    use Ordering::*;
    use Priority::*;
    match (self, other) {
      (REQUIRED, REQUIRED) => Some(Equal),
      (REQUIRED, IMPORTANT) => Some(Greater),
      (REQUIRED, STANDARD) => Some(Greater),
      (REQUIRED, OPTIONAL) => Some(Greater),
      (REQUIRED, EXTRA) => Some(Greater),
      (REQUIRED, UNKNOWN) => Some(Greater),
      (IMPORTANT, REQUIRED) => Some(Less),
      (IMPORTANT, IMPORTANT) => Some(Equal),
      (IMPORTANT, STANDARD) => Some(Greater),
      (IMPORTANT, OPTIONAL) => Some(Greater),
      (IMPORTANT, EXTRA) => Some(Greater),
      (IMPORTANT, UNKNOWN) => Some(Greater),
      (STANDARD, REQUIRED) => Some(Less),
      (STANDARD, IMPORTANT) => Some(Less),
      (STANDARD, STANDARD) => Some(Equal),
      (STANDARD, OPTIONAL) => Some(Greater),
      (STANDARD, EXTRA) => Some(Greater),
      (STANDARD, UNKNOWN) => Some(Greater),
      (OPTIONAL, REQUIRED) => Some(Less),
      (OPTIONAL, IMPORTANT) => Some(Less),
      (OPTIONAL, STANDARD) => Some(Less),
      (OPTIONAL, OPTIONAL) => Some(Equal),
      (OPTIONAL, EXTRA) => Some(Greater),
      (OPTIONAL, UNKNOWN) => Some(Greater),
      (EXTRA, REQUIRED) => Some(Less),
      (EXTRA, IMPORTANT) => Some(Less),
      (EXTRA, STANDARD) => Some(Less),
      (EXTRA, OPTIONAL) => Some(Less),
      (EXTRA, EXTRA) => Some(Equal),
      (EXTRA, UNKNOWN) => Some(Greater),
      (UNKNOWN, REQUIRED) => Some(Less),
      (UNKNOWN, IMPORTANT) => Some(Less),
      (UNKNOWN, STANDARD) => Some(Less),
      (UNKNOWN, OPTIONAL) => Some(Less),
      (UNKNOWN, EXTRA) => Some(Less),
      (UNKNOWN, UNKNOWN) => Some(Equal),
    }
  }
}

impl Default for Priority {
  fn default() -> Self {
    Self::UNKNOWN
  }
}

#[derive(Debug, PartialEq, Copy, Clone, Display)]
pub enum Section {
  ADMIN,
  COMM,
  DATABASE,
  DEBUG,
  DEVEL,
  DOC,
  EDITORS,
  FONTS,
  GAMES,
  GNOME,
  GRAPHICS,
  HTTPD,
  INTERPRETERS,
  INTROSPECTION,
  JAVA,
  KERNEL,
  LIBDEVEL,
  LIBS,
  LISP,
  LOCALIZATION,
  MAIL,
  MATH,
  METAPACKAGES,
  MISC,
  NET,
  OLDLIBS,
  OTHEROSFS,
  PERL,
  PHP,
  PYTHON,
  RUBY,
  SCIENCE,
  SHELLS,
  SOUND,
  TEXT,
  TRANSLATIONS,
  UTILS,
  VCS,
  VIDEO,
  WEB,
  X11,
  ZOPE,
  UNKNOWN,
}

impl Default for Section {
  fn default() -> Self {
    Self::UNKNOWN
  }
}

// XXX should hold version info with '>', '=', '>=', etc...
pub fn parse_depends(_dep: &str) -> Result<(String, Option<String>), String> {
  let dep: String = _dep.trim().to_string();
  if dep.contains(" ") {
    // XXX assumes version format is like: (>=2.32)
    // 0: package, 1: ( and {>=<}, 2: version and )
    let parts = dep.split(" ").collect::<Vec<_>>();
    let pkg = parts[0].trim().split(":").collect::<Vec<_>>()[0];
    let version = &parts[2][..parts[2].len() - 1];
    Ok((pkg.trim().to_string(), Some(version.trim().to_string())))
  } else {
    return Ok((dep.trim().to_string(), None));
  }
}

pub fn choose_package(p1: &SourcePackage, p2: &SourcePackage) -> SourcePackage {
  // check version and priority only
  if p1.priority > p2.priority {
    p1.clone()
  } else if p1.priority < p2.priority {
    p2.clone()
  } else {
    let cmp_res = comp_version(&p1.version, &p2.version);
    if cmp_res > 0 {
      p1.clone()
    } else if cmp_res < 0 {
      p2.clone()
    } else {
      p1.clone() // completely same
    }
  }
}

pub fn resolve_duplication(
  sources: &Vec<SourcePackage>,
  _progress_bar: Option<&ProgressBar>,
) -> Result<Vec<SourcePackage>, String> {
  let mut hashmap: HashMap<String, SourcePackage> = HashMap::new();

  if _progress_bar.is_some() {
    _progress_bar.unwrap().set_length(sources.len() as u64);
    _progress_bar.unwrap().set_position(0);
  }
  for item in sources {
    if _progress_bar.is_some() {
      _progress_bar.unwrap().set_message(item.package.clone());
      _progress_bar.unwrap().inc(1);
    }

    if hashmap.contains_key(&item.package) {
      hashmap.insert(
        item.package.to_owned(),
        choose_package(&item, &hashmap.get(&item.package).unwrap()),
      );
    } else {
      hashmap.insert(item.package.to_owned(), item.to_owned());
    }
  }

  if _progress_bar.is_some() {
    _progress_bar.unwrap().finish_with_message("DONE");
  }

  Ok(hashmap.values().map(|i| i.to_owned()).collect::<Vec<_>>())
}

#[cfg(test)]
pub mod test {
  #[test]
  fn test_package_source_from_row() {
    let sample = std::fs::read_to_string("test/sample-index").unwrap();
    let psources = super::SourcePackage::from_row(&sample).unwrap();
    let dpkg = &psources[0];
    assert_eq!(psources.len(), 3);
    assert_eq!(dpkg.package, "dpkg");
    assert_eq!(dpkg.pre_depends["libzstd1"].as_ref().unwrap(), "1.3.2");
    assert_eq!(dpkg.arch[0], super::Arch::AMD64);
    assert_eq!(dpkg.version, "1.19.7ubuntu3");
    assert_eq!(dpkg.essential, true);
    assert_eq!(dpkg.section, super::Section::ADMIN);
    assert_eq!(
      dpkg.maintainer,
      "Ubuntu Developers <ubuntu-devel-discuss@lists.ubuntu.com>"
    );
    assert_eq!(
      dpkg.original_maintainer,
      "Dpkg Developers <debian-dpkg@lists.debian.org>"
    );
    assert_eq!(dpkg.suggests.contains(&"apt".to_string()), true);
    assert_eq!(dpkg.suggests.contains(&"debsig-verify".to_string()), true);
    assert_eq!(
      dpkg.filename,
      "pool/main/d/dpkg/dpkg_1.19.7ubuntu3_amd64.deb"
    );
    assert_eq!(dpkg.chksum_md5, "f595c79475d3c2ac808eaac389071c35");
    assert_eq!(
      dpkg.description,
      "Debian package management system\nwaiwai second sentence.\nuouo fish life."
    );
  }

  #[test]
  fn test_comp_version_char() {
    use super::comp_version_char;
    assert_eq!(comp_version_char(Some('5'), Some('4')), 1);
    assert_eq!(comp_version_char(Some('3'), Some('3')), 0);
    assert_eq!(comp_version_char(Some('3'), Some('~')), 1);
    assert_eq!(comp_version_char(Some('3'), None), 1);
    assert_eq!(comp_version_char(Some('~'), None), -1);
  }

  #[test]
  fn test_split_in_upstream() {
    use super::split_in_upstream;
    assert_eq!(
      split_in_upstream("34.3.2"),
      vec![
        ("34".to_string(), ".".to_string()),
        ("3".to_string(), ".".to_string()),
        ("2".to_string(), "".to_string())
      ]
    );
    assert_eq!(
      split_in_upstream("34.3build1-ubuntu3~pre"),
      vec![
        ("34".to_string(), ".".to_string()),
        ("3".to_string(), "build".to_string()),
        ("1".to_string(), "-ubuntu".to_string()),
        ("3".to_string(), "~pre".to_string())
      ]
    );
  }

  #[test]
  fn test_package_resolve_duplication() {
    let sample = std::fs::read_to_string("test/sample-duplicated-index").unwrap();
    let psources = super::SourcePackage::from_row(&sample).unwrap();
    assert_eq!(psources.len(), 3);
    let resolved = super::resolve_duplication(&psources, None).unwrap();
    assert_eq!(resolved.len(), 1);
    let dpkg = resolved.iter().nth(0).unwrap();
    assert_eq!(dpkg.package, "dpkg");
    assert_eq!(dpkg.priority, super::Priority::REQUIRED);
    assert_eq!(dpkg.version, "1.20.7ubuntu3");
  }

  #[test]
  fn test_parse_depends() {
    let dep1 = "libc6 (>= 2.15)";
    let dep2 = "libbz2-1.0";
    let pdep1 = super::parse_depends(dep1).unwrap();
    let pdep2 = super::parse_depends(dep2).unwrap();
    assert_eq!(pdep1.0, "libc6");
    assert_eq!(pdep1.1.unwrap(), "2.15");
    assert_eq!(pdep2.0, "libbz2-1.0");
    assert_eq!(pdep2.1, None);
  }
}
