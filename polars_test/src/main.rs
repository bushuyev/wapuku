use std::borrow::Cow;
use std::io::Cursor;
use polars::prelude::*;
use polars::io::parquet::*;



fn main() {
    let parquet_bytes = include_bytes!("../../wapuku-model/data/d2_transactions_pi_message.par");

    let mut buff = Cursor::new(parquet_bytes);
    
    let mut reader = ParquetReader::new(buff);

    println!("ok: entries in batch - {:?} ", reader.num_rows());
    // println!("Ok");
}
