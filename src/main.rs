use std::{
    env,
    ffi::OsStr,
    fs::{self, DirEntry},
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Default, Debug)]
struct DataHtml {
    path_html: String,
    date: String,
    esf: String,
    address: String,
    tables: Vec<TableRow>,
}

#[derive(Default, Debug, Clone)]
struct TableRow {
    number: i32,
    name: String,
    dop_name: String,
    code: u32,
    tarif: f64,
    price: f64,
    size_price: f64,
    percent: String,
    summ: f64,
    price_with: f64,
    declaration: String,
    declaration_num: i32,
    additionally: u64,
}

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

    let mut full_data: Vec<DataHtml> = vec![];

    let htmls = get_files("data/temp/", "html");
    match htmls {
        Ok(htmls) => htmls
            .into_iter()
            .for_each(|file| match file.path().to_str() {
                Some(path) => {
                    let data = get_data_in_html(path);
                    match data {
                        Ok(data) => full_data.push(data),
                        Err(_) => println!("Нет данных"),
                    }
                }
                None => println!("Что-то не так"),
            }),
        Err(_) => println!("Не найдены html-файлы"),
    }
    println!("#######\n#######");
    analyze(full_data);
}

fn analyze(full_data: Vec<DataHtml>) {
    let mut vec_sort_of_address: Vec<Vec<&DataHtml>> = vec![];

    let mut v = full_data
        .iter()
        .map(|element| &element.address)
        .collect::<Vec<_>>();
    v.sort_unstable();
    v.dedup();

    v.iter().for_each(|address| {
        let v = full_data
            .iter()
            .filter(|element| &&element.address == address)
            .collect::<Vec<_>>();
        vec_sort_of_address.push(v);
    });

    let mut products = vec![];
    let _ = vec_sort_of_address[2]
        .iter()
        .map(|data| data.tables.clone())
        .map(|table| table.clone())
        .for_each(|table| {
            table.iter().for_each(|row| {
                products.push(row.name.to_owned());
            });
        });

    dbg!(products);
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

fn get_data_in_html(path: &str) -> Result<DataHtml, std::io::Error> {
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

            let data = DataHtml {
                path_html: path.to_owned(),
                date: get_value(&elements, "Дата выписки", 1, parser)
                    .unwrap_or("Нет даты".to_string()),
                esf: get_value(&elements, "Регистрационный номер", 1, parser)
                    .unwrap_or("-".to_string()),
                address: get_value(&elements, "Адрес доставки", 1, parser)
                    .unwrap_or("Нет адреса".to_string()),
                tables: get_table_rows(&elements, parser),
            };
            println!("{}", &path);
            println!("--#--#--");
            Ok(data)
        }
        Err(e) => {
            println!("Не удалось прочитать файл");
            Err(e)
        }
    }
}

fn get_value(
    elements: &Vec<&tl::Node>,
    value_in_html: &str,
    shift: usize,
    parser: &tl::Parser,
) -> Option<String> {
    let element = elements
        .iter()
        .enumerate()
        .find(|node| node.1.inner_text(parser) == value_in_html);
    match element {
        Some(element) => {
            let (position, _node) = element;
            let value = elements[position + shift].inner_text(parser);
            println!("{}", &value);
            Some(value.to_string())
        }
        None => {
            println!("-Ничего не найдено-");
            None
        }
    }
}

fn get_table_rows(elements: &Vec<&tl::Node>, parser: &tl::Parser) -> Vec<TableRow> {
    let start_position = elements
        .iter()
        .enumerate()
        .find(|node| node.1.inner_text(parser) == "Раздел G. Данные по товарам, работам, услугам");

    let end_position = elements
        .iter()
        .enumerate()
        .find(|node| node.1.inner_text(parser) == "Всего по счету");

    match (start_position, end_position) {
        (None, None) | (None, Some(_)) | (Some(_), None) => {
            println!("--Ничего--");
            let my_values: Vec<TableRow> = vec![];
            my_values
        }
        (Some(start), Some(end)) => {
            let start = start.0 + 7 + 21 + 19;
            let end = end.0;

            let ref_table = elements[start..end]
                .iter()
                .map(|node| node.inner_text(parser))
                .enumerate()
                .collect::<Vec<_>>();

            let rows_start_position = ref_table[0..ref_table.len() - 16]
                .iter()
                .filter(|element| {
                    element.1.parse::<u32>().is_ok()
                        && ref_table[element.0 + 1].1.parse::<u32>().is_ok()
                        && ref_table[element.0 + 2].1.parse::<u32>().is_err()
                        && ref_table[element.0 + 3].1.parse::<u32>().is_err()
                        && ref_table[element.0 + 4].1.parse::<u32>().is_ok()
                        && ref_table[element.0 + 5].1.parse::<u32>().is_err()
                        && ref_table[element.0 + 16].1.parse::<u32>().is_ok()
                })
                .collect::<Vec<_>>();

            let vec_t_row: Vec<TableRow> = rows_start_position
                .iter()
                .map(|element| {
                    let mut t_row = TableRow {
                        ..Default::default()
                    };

                    let start = element.0;
                    t_row.number = ref_table[start].1.parse::<i32>().unwrap_or_default();
                    t_row.name = ref_table[start + 2].1.parse::<String>().unwrap_or_default();
                    t_row.dop_name = ref_table[start + 3].1.parse::<String>().unwrap_or_default();
                    t_row.code = ref_table[start + 4].1.parse::<u32>().unwrap_or_default();

                    let tarif_rep = ref_table[start + 6].1.replace(",", ".");
                    t_row.tarif = tarif_rep
                        .parse::<f64>()
                        .unwrap_or_else(|_| tarif_rep.parse::<f32>().unwrap_or_default() as f64);

                    let price_rep = ref_table[start + 7].1.replace(",", ".");
                    t_row.price = price_rep
                        .parse::<f64>()
                        .unwrap_or_else(|_| tarif_rep.parse::<f32>().unwrap_or_default() as f64);

                    let size_price_rep = ref_table[start + 10].1.replace(",", ".");
                    t_row.size_price = size_price_rep
                        .parse::<f64>()
                        .unwrap_or_else(|_| tarif_rep.parse::<f32>().unwrap_or_default() as f64);

                    t_row.percent = ref_table[start + 11]
                        .1
                        .parse::<String>()
                        .unwrap_or_default();

                    let summ_rep = ref_table[start + 12].1.replace(",", ".");
                    t_row.summ = summ_rep
                        .parse::<f64>()
                        .unwrap_or_else(|_| tarif_rep.parse::<f32>().unwrap_or_default() as f64);

                    let price_with_rep = ref_table[start + 13].1.replace(",", ".");
                    t_row.price_with = price_with_rep
                        .parse::<f64>()
                        .unwrap_or_else(|_| tarif_rep.parse::<f32>().unwrap_or_default() as f64);

                    t_row.declaration = ref_table[start + 14]
                        .1
                        .parse::<String>()
                        .unwrap_or_default();

                    t_row.declaration_num =
                        ref_table[start + 15].1.parse::<i32>().unwrap_or_default();

                    t_row.additionally = ref_table[start + 17].1.parse::<u64>().unwrap_or_default();

                    t_row
                })
                .collect::<Vec<_>>();

            vec_t_row
        }
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
