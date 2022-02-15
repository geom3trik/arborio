use include_dir::{include_dir, Dir, DirEntry};
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use crate::celeste_mod::walker::ConfigSourceTrait;

const EMBEDDED: Dir = include_dir!("conf");
#[derive(Copy, Clone)]
pub struct EmbeddedSource();

impl ConfigSourceTrait for EmbeddedSource {
    fn filesystem_root(&mut self) -> Option<PathBuf> {
        None
    }

    fn list_dirs(&mut self, path: &Path) -> Box<dyn Iterator<Item = PathBuf>> {
        Box::new(
            EMBEDDED
                .get_dir(path)
                .map(Dir::dirs)
                .into_iter()
                .flatten()
                .map(Dir::path)
                .map(Path::to_owned),
        )
    }

    fn list_all_files(&mut self, path: &Path) -> Box<dyn Iterator<Item = PathBuf>> {
        Box::new(
            EMBEDDED
                .get_dir(path)
                .map(EmbeddedFileIter::new)
                .into_iter()
                .flatten()
                .map(Path::to_path_buf),
        )
    }

    fn get_file(&mut self, path: &Path) -> Option<Box<dyn Read>> {
        Some(Box::new(Cursor::new(EMBEDDED.get_file(path)?.contents())))
    }
}

struct EmbeddedFileIter<'a> {
    stack: Vec<(&'a Dir<'a>, usize)>,
}

impl<'a> EmbeddedFileIter<'a> {
    fn new(start: &'a Dir) -> Self {
        Self {
            stack: vec![(start, 0)],
        }
    }
}

impl<'a> Iterator for EmbeddedFileIter<'a> {
    type Item = &'a Path;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((cur_dir, cur_idx)) = self.stack.last_mut() {
            if let Some(entry) = cur_dir.entries().get(*cur_idx) {
                *cur_idx += 1;
                match entry {
                    DirEntry::Dir(sub_dir) => self.stack.push((sub_dir, 0)),
                    DirEntry::File(file) => return Some(file.path()),
                }
            } else {
                self.stack.pop();
            }
        }

        None
    }
}
