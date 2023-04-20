use std::any::Any;
use std::borrow::Cow;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Cursor};
use std::sync::Arc;

use polars::prelude::*;
use polars::io::parquet::*;

// use arrow::datatypes::{DataType, Field, Int32Type, Schema, SchemaRef, TimeUnit};
use bytes::Bytes;

use log::{debug, trace};
// use parquet::arrow::parquet_to_arrow_schema;
// use parquet::file::footer::{decode_footer, parse_metadata};
// use parquet::schema::types::SchemaDescriptor;



pub fn parquet_scan(){
    let parquet_bytes = include_bytes!("../../wapuku-model/data/d2_transactions_pi_message.par");
    
    let mut buff = Cursor::new(parquet_bytes);
    
    let mut reader = ParquetReader::new(buff);
    
    debug!("parquet_scan: metadata={:?}", reader.num_rows());

}