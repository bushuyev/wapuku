use std::borrow::Cow;
use std::io::Cursor;
use polars::prelude::*;
use polars::io::parquet::*;



fn main() {

    let df = df!(
        "field_1" => &[10,      20,     30,     40,   41,    50,     60,     70,     80,     90], 
        "field_2" => &[1,       1,      1,      1,    3,     2,      2,      2,      2,      2],
        "field_3" => &["a",     "b",    "c",    "d",  "dd",   "e",    "f",    "g",    "h",    "i"]
    ).unwrap();
    // let parquet_bytes = include_bytes!("../../wapuku-model/data/d2_transactions_pi_message.par");
    // 
    // let mut buff = Cursor::new(parquet_bytes);
    // 
    // let mut reader = ParquetReader::new(buff);
    // 
    // println!("ok: entries in batch - {:?} ", reader.num_rows());
    println!("Okk");
}
