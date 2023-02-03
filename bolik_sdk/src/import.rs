use std::{
    fs::DirEntry,
    io::Read,
    path::{Path, PathBuf},
};

use anyhow::{bail, Result};

use crate::{
    account::AccView,
    import::v2::ImportCardResult,
    registry::{WithAccountAtom, WithDeviceAtom, WithDocsAtom, WithTimelineAtom, WithTxn},
};

mod v1;
mod v2;

#[derive(Default, Clone)]
pub struct ImportResult {
    pub imported: u32,
    /// List of file names that were already imported.
    pub duplicates: Vec<String>,
    /// List of file names that failed.
    pub failed: Vec<String>,
}

pub trait ImportCtx<'a>:
    WithTxn<'a> + WithAccountAtom + WithTimelineAtom + WithDocsAtom + WithDeviceAtom
{
}
impl<'a, T> ImportCtx<'a> for T where
    T: WithTxn<'a> + WithAccountAtom + WithTimelineAtom + WithDocsAtom + WithDeviceAtom
{
}

#[derive(Clone)]
pub struct ImportAtom {}

impl ImportAtom {
    pub fn new() -> Self {
        Self {}
    }

    pub fn run<'a>(&self, ctx: &impl ImportCtx<'a>, in_dir: PathBuf) -> Result<ImportResult> {
        let mut acc = ctx.account().require_account(ctx)?;

        tracing::info!("Starting import from {}", in_dir.display());
        // Read the dir
        let md_files = std::fs::read_dir(&in_dir)?;
        let mut res = ImportResult::default();

        for md_file in md_files {
            let Ok(md_file) = md_file else {
                continue;
            };

            let name = md_file.file_name().to_string_lossy().to_string();
            match self.process_single(ctx, &in_dir, &mut acc, md_file) {
                Ok(ImportSingleResult::Imported) => res.imported += 1,
                Ok(ImportSingleResult::Duplicate) => {
                    res.duplicates.push(name);
                }
                Ok(ImportSingleResult::Directory) => {}
                Err(err) => {
                    tracing::warn!("Failed to process file {}: {}", name, err);
                    res.failed.push(name);
                }
            }
        }

        Ok(res)
    }

    fn process_single<'a>(
        &self,
        ctx: &impl ImportCtx<'a>,
        in_dir: &Path,
        acc: &mut AccView,
        md_file: DirEntry,
    ) -> Result<ImportSingleResult> {
        if md_file.file_type()?.is_dir() {
            return Ok(ImportSingleResult::Directory);
        }

        // Try to read markdown file
        tracing::info!("Importing {}", md_file.path().display());
        let mut file = std::fs::File::open(md_file.path())?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;

        let mut lines = content.split('\n');
        let first_line = lines.next();
        let res = match first_line {
            Some("# Bolik card") => {
                let _res = v1::import_card(ctx, &in_dir, acc, &content)?;
                ImportSingleResult::Imported
            }
            Some("# Bolik card v2") => match v2::import_card(ctx, &in_dir, acc, &content)? {
                ImportCardResult::Imported(_) => ImportSingleResult::Imported,
                ImportCardResult::Duplicate => ImportSingleResult::Duplicate,
            },
            _ => {
                bail!("Unsupported card {}", md_file.path().display());
            }
        };
        Ok(res)
    }
}

enum ImportSingleResult {
    Imported,
    Duplicate,
    Directory,
}
