use std::path::Path;
use walkdir::WalkDir;

use crate::assets::InternedMap;
use crate::celeste_mod::everest_yaml::{arborio_module_yaml, celeste_module_yaml, EverestYaml};
use crate::celeste_mod::module::CelesteModule;
use crate::celeste_mod::walker::{
    open_module, ConfigSource, ConfigSourceTrait, EmbeddedSource, FolderSource,
};

pub fn for_each_mod<F: FnMut(usize, usize, &str, ConfigSource)>(root: &Path, mut callback: F) {
    let to_load = WalkDir::new(root.join("Mods"))
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .collect::<Vec<_>>();
    let total = to_load.len();

    for (i, entry) in to_load.iter().enumerate() {
        if let Some(config) = open_module(entry.path()) {
            let name = entry
                .path()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap_or("<bad unicode>");
            callback(i, total, name, config);
        }
    }
}

pub fn load_all<F>(root: &Path, modules: &mut InternedMap<CelesteModule>, mut progress: F)
where
    F: FnMut(f32, String),
{
    let mut total = 0.0;
    for_each_mod(root, |i, n, name, config| {
        let i = i as f32;
        let n = n as f32 + 2.0;
        total = n;
        progress(i / n, format!("Loading {}", name));
        load_into(config, modules);
    });

    progress(total / (total + 2.0), "Loading Celeste".to_owned());
    modules.insert("Celeste".into(), {
        let path = root.join("Content");
        let source = FolderSource::new(&path).unwrap();
        let mut r = CelesteModule::new(Some(path), celeste_module_yaml());
        r.load(&mut source.into());
        r
    });
    progress(
        (total + 1.0) / (total + 2.0),
        "Loading built-in config".to_owned(),
    );
    modules.insert("Arborio".into(), {
        let source = EmbeddedSource();
        let mut r = CelesteModule::new(None, arborio_module_yaml());
        r.load(&mut source.into());
        r
    });
}

pub fn load_into(mut source: ConfigSource, modules: &mut InternedMap<CelesteModule>) {
    if let Some(mut reader) = source.get_file(Path::new("everest.yaml")) {
        let mut data = String::new();
        reader.read_to_string(&mut data).unwrap();
        let everest_yaml: Vec<EverestYaml> =
            match serde_yaml::from_str(data.trim_start_matches('\u{FEFF}')) {
                Ok(e) => e,
                Err(e) => {
                    println!(
                        "Error parsing {}/everest.yaml: {:?}",
                        source
                            .filesystem_root()
                            .unwrap()
                            .to_str()
                            .unwrap_or("<invalid unicode>"),
                        e
                    );
                    return;
                }
            };
        if everest_yaml.len() != 1 {
            println!(
                "Error parsing {}/everest.yaml: {} entries",
                source
                    .filesystem_root()
                    .unwrap()
                    .to_str()
                    .unwrap_or("<invalid unicode>"),
                everest_yaml.len()
            );
        }
        let mut module = CelesteModule::new(
            source.filesystem_root(),
            everest_yaml.into_iter().next().unwrap(),
        );
        module.load(&mut source);
        modules.insert(module.everest_metadata.name, module);
    }
}
