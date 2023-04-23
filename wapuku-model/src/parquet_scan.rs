use std::any::Any;
use std::borrow::Cow;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Cursor};
use std::sync::Arc;

use polars::prelude::*;
use polars::time::*;
// use polars::time::windows::*;
// use polars::prelude::windows::group_by::*;
use polars::io::parquet::*;
use polars::lazy::*;
// use arrow::datatypes::{DataType, Field, Int32Type, Schema, SchemaRef, TimeUnit};
use bytes::Bytes;

use log::{debug, trace};
// use parquet::arrow::parquet_to_arrow_schema;
// use parquet::file::footer::{decode_footer, parse_metadata};
// use parquet::schema::types::SchemaDescriptor;
use smartstring::alias::String as SmartString;

pub fn parquet_scan() {
    let parquet_bytes = include_bytes!("../../wapuku-model/data/s1_transactions_pi_message.par");
    
    let mut buff = Cursor::new(parquet_bytes);
    
    let mut df = ParquetReader::new(buff)
        .finish().unwrap()
        .lazy()
        .groupby([col("PAYMENTSTATUS")])
        .agg([count()])
        .sort("count", SortOptions {
            descending: true,
            nulls_last: true,
            multithreaded: true,
        },)
        .collect()
        .unwrap();
  
    debug!("parquet_scan: height={:?}", df.height());

}


pub fn group_by() {
    let df = df!("field_1" => &[10, 20, 30, 40], "field_2" => &[1, 2, 3, 4]).unwrap();

    let min_max = df.clone().lazy().select([
        col("field_1").min().alias("field_1_min"),
        col("field_1").max().alias("field_1_max")
    ]).collect();
    
    // let df = df.clone()
    //     .lazy()
        // .groupby([col("field_2").lt(20)])
        // .groupby([10])
        // .agg([sum("field_2")])
        // .collect();
    // debug!("parquet_scan: df={:?} min_max={:?}", df, min_max);

    let df = df.clone()
        .lazy()
        .groupby_dynamic(
            [], 
            DynamicGroupOptions {
                index_column: "field_1".into(),
                every: Duration::new(20),
                period: Duration::new(20),
                offset: Duration::new(0),
                truncate: true,
                include_boundaries: true,
                closed_window: ClosedWindow::Left,
                start_by: Default::default(),
         }
        )
        .agg([sum("field_2")])
        .collect();
    
    debug!("parquet_scan: df={:?}", df);

    // debug!("parquet_scan: time_key={:?}, keys={:?}, groups={:?}", time_key, keys, groups);
    
}