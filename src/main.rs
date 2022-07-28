use std::{
    env,
    ffi::OsStr,
    fs::{self, DirEntry},
    path::{Path, PathBuf},
    process::Command,
};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        println!("The first argument is {}", args[1]);
        if args[1] != "skip" {
            init_folders_struct();
            convert_all_pdfs();
        }
    } else {
        init_folders_struct();
        convert_all_pdfs();
    }

    let htmls = get_files("data/temp/", "html");
    match htmls {
        Ok(htmls) => htmls
            .into_iter()
            .for_each(|file| match file.path().to_str() {
                Some(path) => get_data_in_html(path),
                None => println!("Что-то не так"),
            }),
        Err(_) => println!("Не найдены html-файлы"),
    }
}

fn convert_all_pdfs() {
    let pdfs = get_files("data/original_pdf/", "pdf");
    match pdfs {
        Ok(pdfs) => pdfs
            .into_iter()
            .for_each(|file| match file.path().to_str() {
                Some(path) => pdf_to_html_convert(path),
                None => println!("Что-то не так"),
            }),
        Err(_) => println!("Не найдены pdf-файлы"),
    }
}

fn get_data_in_html(path: &str) {
    let html = fs::read_to_string(path);
    match html {
        Ok(html) => {
            let dom = tl::parse(&html, tl::ParserOptions::default()).unwrap();
            let parser = dom.parser();
            let elements = dom
                .get_elements_by_class_name("c")
                .filter_map(|node_handle| node_handle.get(parser))
                .filter(|node| node.as_tag().map_or(false, |tag| tag.name() == "div"))
                .collect::<Vec<_>>();

            let element = elements
                .iter()
                .enumerate()
                .find(|node| node.1.inner_text(parser) == "Адрес доставки");

            match element {
                Some(element) => {
                    let (position, _node) = element;
                    let address = elements[position + 1].inner_text(parser);
                    println!("{}", &address);
                    sort_pdfs_on_folder(&address, path);
                }
                None => println!("Ничего не найдено"),
            }
        }
        Err(_) => println!("Не удалось прочитать файл"),
    }
}

fn sort_pdfs_on_folder(address: &str, html_path: &str) {
    let raw_folder_name = address.to_string();
    let folder_name = raw_folder_name.replace("/", "_");
    let mut dir: String = "data/sort_pdf/".to_string();
    dir.push_str(&folder_name);
    fs::create_dir_all(dir).expect("Не удалось создать каталоги");

    let html_path = Path::new(html_path);

    match html_path.file_stem() {
        Some(name_without_extension) => match name_without_extension.to_str() {
            Some(name_without_extension) => {
                let mut filename_with_ext: String = name_without_extension.to_owned();
                filename_with_ext.push_str(".pdf");

                let mut original_path = PathBuf::from("data/original_pdf/");
                original_path.push(&filename_with_ext);

                let mut copy_path = PathBuf::new();
                copy_path.push("data/sort_pdf/");
                copy_path.push(folder_name);
                copy_path.push(&filename_with_ext);

                fs::copy(original_path, copy_path).expect("Копировать не удалось");
            }
            None => println!("Не удалось получить имя файла"),
        },
        None => println!("Не удалось получить имя файла"),
    }
}

fn pdf_to_html_convert(path: &str) {
    if cfg!(target_os = "windows") {
        let com = Command::new("data/pdf2htmlex/pdf2htmlex")
            .args([path, "--dest-dir=./data/temp"])
            .output()
            .expect("failed to execute process");

        println!(
            "status: {} {}",
            if com.status.code() == Some(0i32) {
                "OK"
            } else {
                "ERR"
            },
            path
        );
    };
}

fn get_files(path: &str, file_type: &str) -> Result<Vec<DirEntry>, std::io::Error> {
    let files = fs::read_dir(path);
    match files {
        Ok(files) => {
            let pdf_vec = files
                .into_iter()
                .filter_map(Result::ok)
                .filter(|e| e.path().extension() == Some(OsStr::new(file_type)))
                .collect::<Vec<_>>();
            Ok(pdf_vec)
        }
        Err(e) => Err(e),
    }
}

fn init_folders_struct() {
    fs::create_dir_all("data/original_pdf/").expect("Не удалось создать каталоги");
    fs::create_dir_all("data/pdf2htmlex/").expect("Не удалось создать каталоги");
    if Path::new("data/temp/").exists() {
        fs::remove_dir_all("data/temp/").expect("-");
    }
    fs::create_dir_all("data/temp/").expect("-");
    if Path::new("data/sort_pdf/").exists() {
        fs::remove_dir_all("data/sort_pdf/").expect("-");
    }
    fs::create_dir_all("data/sort_pdf/").expect("Не удалось создать каталоги");
}
