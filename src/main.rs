use asar::{AsarReader, AsarWriter, Result};
use std::env;
use std::{fs, fs::File, path::PathBuf};

#[derive(Debug)]
enum PatchType {
    Bundles(PathBuf),
    Resources(PathBuf),
    None,
}

fn main() -> Result<()> {
    let usernames_colors = [
        "--cpd-color-blue-900",
        "--cpd-color-green-900",
        "--cpd-color-pink-900",
        "--cpd-color-purple-900",
        "--cpd-color-cyan-900",
        "--cpd-color-orange-900",
    ];

    let path = &env::args().skip(1).next().or(Some("".to_string())).unwrap();
    let mut patch_type = PatchType::None;

    let element_path = fs::read_dir(path);
    if element_path.is_ok() && (path.contains("Element") || path.contains("element")) {
        let mut app_version = "0.0".to_string();
        let mut app_path = PathBuf::from("");
        for file in element_path.unwrap() {
            if file.is_ok() {
                let file = file.unwrap().path();
                if file.ends_with("resources") {
                    patch_type = PatchType::Resources(file);
                    break;
                } else if file.ends_with("bundles") {
                    patch_type = PatchType::Bundles(file);
                    break;
                } else if file.to_str().unwrap().starts_with("app-") {
                    let file_path = file
                        .strip_prefix(path)
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .replace("app-", "");
                    let file_path_split = file_path.split('.');
                    let mut app_version_split = app_version.split('.');
                    for new_num in file_path_split {
                        let current_num = app_version_split.next();
                        if current_num.is_some() {
                            let new_num: usize = new_num.parse().unwrap();
                            let current_num: usize = current_num.unwrap().parse().unwrap();
                            if current_num == new_num {
                                continue;
                            } else if current_num > new_num {
                                break;
                            }
                        }
                        app_version = file_path;
                        app_path = file;
                        break;
                    }
                }
            }
        }
        if app_version != "0.0" {
            app_path.push("resources");
            patch_type = PatchType::Resources(app_path);
        }

        match patch_type {
            PatchType::None => (),
            PatchType::Bundles(path) => {
                let path = path.read_dir().unwrap();
                let mut bundle_folder = find_newest_bundle_folder(path);
                bundle_folder.push("theme-dark.css");
                let path = bundle_folder;
                println!("Patching: {:?}", path.to_str().unwrap());
                patch_theme_file(&path.to_str().unwrap(), usernames_colors)?;
                return Ok(());
            }
            PatchType::Resources(mut path) => {
                path.push("webapp.asar");
                println!("Patching: {:?}", path.to_str().unwrap());
                patch_asar(path.to_str().unwrap(), usernames_colors)?;
                return Ok(());
            }
        }
    }
    println!("Please specify path to main element directory.");
    println!("Possible paths:");
    println!("\tFlatpak: /var/lib/flatpak/app/im.riot.Riot/current/active/files/Element/");
    println!("\tWindows: C:/Users/<username>/AppData/Local/element-desktop/");
    println!("\tarch: /usr/share/webapps/element/");
    Ok(())
}
fn patch_theme_file(path: &str, usernames_colors: [&str; 6]) -> Result<()> {
    // backup
    fs::copy(path, path.replace(".css", "_backup.css"))?;

    let mut theme_file = fs::read_to_string(path)?;

    for (i, color) in usernames_colors.iter().enumerate() {
        theme_file = replace_username_color(&theme_file, i, color);
    }

    fs::remove_file(path)?;
    fs::write(path, theme_file)?;

    Ok(())
}

fn patch_asar(path: &str, usernames_colors: [&str; 6]) -> Result<()> {
    // backup
    fs::copy(path, path.replace(".asar", "_backup.asar"))?;

    let asar_file = fs::read(path)?;
    let reader = AsarReader::new(&asar_file, PathBuf::from(path))?;

    let mut writer = AsarWriter::new();

    // writer.add_from_reader(&reader)?;
    for (path, file) in reader.files() {
        if path.ends_with("theme-dark.css") && path.starts_with("bundles") {
            let mut file_content = std::str::from_utf8(file.data()).unwrap().to_string();

            for (i, color) in usernames_colors.iter().enumerate() {
                file_content = replace_username_color(&file_content, i, color);
            }

            writer.write_file(path, file_content, false)?;
        } else {
            writer.write_file(path, file.data(), false)?;
        }
    }

    fs::remove_file(path)?;
    writer.finalize(File::create(path)?)?;

    Ok(())
}

fn replace_username_color(file_content: &str, user_number: usize, color: &str) -> String {
    let user_number = user_number + 1;
    let to_find_start = &format!("--cpd-color-text-decorative-{user_number}:var(");
    let start_index = file_content.find(to_find_start).unwrap();
    let to_find_end = ");";
    let end_index =
        start_index + file_content[start_index..].find(to_find_end).unwrap() + to_find_end.len();
    let text = file_content.get(start_index..end_index).unwrap();

    let file_content = file_content.replace(text, &format!("{to_find_start}{color}{to_find_end}"));

    file_content
}

fn find_newest_bundle_folder(bundles_dir: std::fs::ReadDir) -> std::path::PathBuf {
    let mut bundles_dir = bundles_dir
        .filter_map(|entry| entry.ok())
        .filter(|file| {
            file.file_type()
                .map_or(false, |file_type| file_type.is_dir())
        })
        .map(|dir| {
            (
                dir.path(),
                dir.path()
                    .read_dir()
                    .map_or(0, |dir_reader| dir_reader.count()),
            )
        });

    bundles_dir.find(|(_, count)| *count > 5).unwrap().0
}
