use std::any::Any;
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufReader, Cursor};
use std::marker::PhantomData;
use std::sync::Arc;

use bytes::Bytes;
use log::{debug, trace};
// use polars::time::windows::*;
// use polars::prelude::windows::group_by::*;
use polars::io::parquet::*;
use polars::lazy::*;
use polars::prelude::*;
use polars::prelude::Expr::Columns;
use polars::time::*;
use polars::time::Duration;
// use parquet::arrow::parquet_to_arrow_schema;
// use parquet::file::footer::{decode_footer, parse_metadata};
// use parquet::schema::types::SchemaDescriptor;
use smartstring::alias::String as SmartString;

use crate::model::*;
use crate::data_type::WapukuDataType;

impl From<PolarsError> for WapukuError {
    fn from(value: PolarsError) -> Self {
        WapukuError::DataFrame{msg: value.to_string()}
    }
}



pub struct PolarsData {
    df:DataFrame,
    property_sets: Vec<SimplePropertiesSet>,
}

impl PolarsData {

    pub fn new(df:DataFrame) -> Self {//TODO move to add df
        // parquet_scan();



        let properties = df.schema().iter_fields().map(|f| DataProperty::new(WapukuDataType::Numeric, f.name)).collect();
        Self {
            df,
            property_sets: vec![SimplePropertiesSet::new(
                // vec![
                //     DataProperty::new(WapukuDataType::Numeric, "property_1"),
                //     DataProperty::new(WapukuDataType::Numeric, "property_2"),
                //     DataProperty::new(WapukuDataType::Numeric, "property_3"),
                // ],
                properties,
                "item_1",
            )], 
        }
    }
}

impl Data for PolarsData {

    fn all_sets(&self) -> Vec<&dyn PropertiesSet> {
        self.property_sets.iter().fold(vec![], |mut props, p| {
            props.push(p);

            props
        })
    }

    fn all_properties(&self) -> HashSet<&dyn Property> {
        self.property_sets.iter().flat_map(|property_set|property_set.properties().into_iter()).collect()
    }

    fn group_by_1(&self, property_range: PropertyRange) -> GroupsVec {

        // GroupsVec::new(property_range.property().clone_to_box(), vec![
        //     // Box::new(SimpleDataGroup::new(10, vec![], DataBounds::X(property_range.to_range(Some(0.0),Some(10.0)))))
        // ])
        todo!()
    }

    fn group_by_2(&self, property_x: PropertyRange, property_y: PropertyRange, x_n: u8, y_n: u8) -> GroupsGrid {
        let property_x_name = property_x.property().name().as_str();
        let property_y_name = property_y.property().name().as_str();

        let properties_df = self.df.select([property_x_name, property_y_name]).unwrap();
        let min_df = properties_df.min();
        let max_df = properties_df.max();
        
        let property_x_step = (max_df.column(property_x_name).unwrap().get(0).unwrap().try_extract::<f32>().unwrap() - min_df.column(property_x_name).unwrap().get(0).unwrap().try_extract::<f32>().unwrap())/(x_n as f32);
        let property_y_step = (max_df.column(property_y_name).unwrap().get(0).unwrap().try_extract::<f32>().unwrap() - min_df.column(property_y_name).unwrap().get(0).unwrap().try_extract::<f32>().unwrap())/(y_n as f32);

        debug!("max_df={:?}, min_df={:?} property_x_step={:?}, property_y_step={:?}", max_df, min_df, property_x_step, property_y_step);

        let mut df = group_by(&self.df,
                          property_x_name, property_x_step.ceil() as i64,
                          property_y_name, property_y_step.ceil() as i64,
                          [
                  col("property_3").count().alias("property_3_count"),
              ]
        ).unwrap();
        
        debug!("df={:?}", df);

        df.as_single_chunk_par();
        let mut iters = df.columns(["primary_field_group", "secondary_group_field", "property_3_count"]).unwrap()
            .iter().map(|s| {
            // debug!("s={:?}", s);
            s.iter()
        }).collect::<Vec<_>>();

        let mut primary_group:Option<i32> = None;
        let mut secondary_group:Option<i32> = None;

        let mut groups_hash:HashMap<i64, HashMap<i64, Vec<AnyValue>>> = HashMap::new();

        for row in 0..df.height() {
            // for iter in &mut iters {
            //     let value = iter.next().expect("should have as many iterations as rows");
            //     debug!("value={:?}", value);
            // }

            let mut row_vec = iters.iter_mut().map(|i|i.next().unwrap()).collect::<Vec<_>>();

            let mut y_hashmap = groups_hash.entry(row_vec[0].try_extract::<i64>().unwrap()).or_insert(HashMap::new());
            y_hashmap.entry(row_vec[1].try_extract::<i64>().unwrap()).or_insert(row_vec[2..].to_vec());

            debug!("row={:?}", row_vec);
        }

        GroupsGrid::new(
            property_x.property().clone_to_box(),
            property_y.property().clone_to_box(),

            (0..x_n).map(|x|
                (0..y_n).map(|y| {
                    let x_0 = (x as f32 * property_x_step).ceil() as i64;
                    let x_1 = (x as f32 * property_x_step + property_x_step).ceil()  as i64;
                    let y_0 = (y as f32 * property_y_step).ceil()  as i64;
                    let y_1 = (y as f32 * property_y_step + property_y_step).ceil() as i64;
                    
                    debug!("x={:?}, y={:?}", (x_0, x_1), (y_0, y_1));

                    Box::<dyn DataGroup>::from(Box::new(SimpleDataGroup::new(groups_hash.get(&x_0).and_then(|h|h.get(&y_0)).map(|v|v.len()).unwrap_or(0), vec![],
                         DataBounds::XY(property_x.to_range(Some(x_0), Some(x_1)), property_y.to_range(Some(y_0), Some(y_1))),
                        )))
                    }
                ).collect::<Vec<Box<dyn DataGroup>>>()

            ).collect::<Vec<Vec<Box<dyn DataGroup>>>>()

        )
    }

}

//TODO move to resources?
pub fn parquet_scan() -> DataFrame {
    // let parquet_bytes = include_bytes!("../../wapuku-model/data/s1_transactions_pi_message.par");
    let parquet_bytes = include_bytes!("../../wapuku-model/data/d2_transactions_pi_message.par");
    
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

    df
}



pub(crate) fn group_by<E: AsRef<[Expr]>>(df:&DataFrame, primary_group_by_field: &str, primary_step: i64, secondary_group_by_field: &str, secondary_step: i64, aggregations: E) -> WapukuResult<DataFrame> {
    let primary_field_value = "primary_field_value"; 
    let primary_field_group = "primary_field_group"; //expanded value of the field in the second column group, to be joined back to main frame

    let primary_field_grouped_and_expanded = df.clone()
        .lazy()
        .select(&[
            col(primary_group_by_field).alias(primary_field_group)
        ])
        .sort(primary_field_group, Default::default())
        .groupby_dynamic([], 
 DynamicGroupOptions {
            index_column: primary_field_group.into(),
            every: Duration::new(primary_step),
            period: Duration::new(primary_step),
            offset: Duration::new(0),
            truncate: true,
            include_boundaries: true,
            closed_window: ClosedWindow::Left,
            start_by: Default::default(),
        }
    ).agg([
        col(primary_field_group).alias(primary_field_value)
    ]).explode([primary_field_value]).collect()?;//primary_field_in_group not used, for debug


    debug!("field2_grouped: field2_grouped={:?}", primary_field_grouped_and_expanded);
    let mut df = df.sort([primary_group_by_field], false)?;
    
    let df = df
        .with_column(primary_field_grouped_and_expanded.column(primary_field_group).unwrap().clone())?
        // .with_column({
        //     let mut series = primary_field_grouped_and_expanded.column("_lower_boundary").unwrap().clone();
        //     series.rename("primary_lower_boundary");
        //     series
        // })?
    ;
    let mut df = df.sort([secondary_group_by_field], false)?;
    
    // let mut df = df.left_join(&primary_field_grouped_and_expanded, [primary_group_by_field], ["primary_field_value"] )?;

    
    debug!("field2_grouped: df={:?}", df);
    
    let mut df = df.clone()
        .lazy()
        .groupby_dynamic(
            [col(primary_field_group)],
            DynamicGroupOptions {
                index_column: secondary_group_by_field.into(),
                every: Duration::new(secondary_step),
                period: Duration::new(secondary_step),
                offset: Duration::new(0),
                truncate: true,
                include_boundaries: true,
                closed_window: ClosedWindow::Left,
                start_by: Default::default(),
         }
        )
        .agg(aggregations)
        .collect()?;
    
    df.rename(secondary_group_by_field, "secondary_group_field");
    
    debug!("parquet_scan: df={:?}", df);

    Ok(df)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use log::debug;
    use polars::datatypes::AnyValue::List;
    use polars::df;
    use polars::prelude::*;
    use crate::model::{Data, Property, PropertyRange};

    use crate::polars_df::{group_by, PolarsData};
    use crate::tests::init_log;

    #[ctor::ctor]
    fn init() {
        std::env::set_var("POLARS_FMT_MAX_ROWS", "100");
        std::env::set_var("FMT_MAX_COLS", "1000");
        
        init_log();
    }
    
    #[test]
    fn test_polars_data(){
        let mut data = PolarsData::new(df!(
            "property_1" => &(0..10).into_iter().collect::<Vec<i64>>(),
            "property_2" => &(0..10).into_iter().map(|i|i*10).collect::<Vec<i64>>(),
            "property_3" => &(0..10).into_iter().map(|i|i).collect::<Vec<i32>>(),
        ).unwrap());

        let all_properties:HashSet<&dyn Property> = data.all_properties();

        let (property_1, property_2, property_3) = {
            let mut all_properties_iter = all_properties.into_iter().collect::<Vec<&dyn Property>>();
            all_properties_iter.sort_by(|p1, p2|p1.name().cmp(p2.name()));

            (*all_properties_iter.get(0).expect("property_1"), *all_properties_iter.get(1).expect("property_2"), *all_properties_iter.get(2).expect("property_3"))
        };

        let mut data_grid = data.group_by_2(
            PropertyRange::new (property_1,  None, None ),
            PropertyRange::new (property_2,  None, None ),
            3, 3
        );

        let data = data_grid.data();

        // assert_eq!(3, data[0][0].volume());
        // assert_eq!(3, data[0][1].volume());
        // assert_eq!(3, data[0][2].volume());
        
        debug!(" data[0][0].volume()={:?}",  data[0][0].volume());
        debug!(" data[0][1].volume()={:?}",  data[0][1].volume());
        debug!(" data[0][2].volume()={:?}",  data[0][2].volume());

        debug!(" data[1][0].volume()={:?}",  data[1][0].volume());
        debug!(" data[1][1].volume()={:?}",  data[1][1].volume());
        debug!(" data[1][2].volume()={:?}",  data[1][2].volume());

        debug!(" data[2][0].volume()={:?}",  data[2][0].volume());
        debug!(" data[2][1].volume()={:?}",  data[2][1].volume());
        debug!(" data[2][2].volume()={:?}",  data[2][2].volume());
    }

    #[test]
    fn test_group_by_2(){
        let mut df = df!(
            "property_1" => &[10,      20,     30,     40,    41,     50,      60,      70,      80,      90], 
            "property_2" => &[1,       1,      1,      1,     3,      2,       2,       2,       2,       2],
            "property_3" => &["a",     "b",    "c",    "d",   "dd",   "e",     "f",     "g",     "h",     "ii"],
            "property_4" => &[100,     200,    300,    400,   410,    500,     600,     700,     800,     900],
            "property_5" => &[1000,    2000,   3000,   4000,  4100,   5000,    6000,    7000,    8000,    9000]
        ).unwrap();
        
        
        debug!("{:?}", df);
        
        let mut polars_data = PolarsData::new(df);

        let all_sets = polars_data.all_sets();
        
        debug!("all_sets={:?}", all_sets);
        assert_eq!(all_sets.len(), 1);
        
        let all_properties = polars_data.all_properties();
        assert_eq!(all_properties.len(), 5);

        let (property_1, property_2, property_3) = {
            let mut all_properties_iter = all_properties.into_iter();

            (all_properties_iter.next().expect("property_1"), all_properties_iter.next().expect("property_2"), all_properties_iter.next().expect("property_3"))
        };

        let mut data_grid = polars_data.group_by_2(
            PropertyRange::new (property_1,  None, None ),
            PropertyRange::new (property_2,  None, None ),
            3, 3
        );
        
        debug!("data_grid={:?}", data_grid.data());
        
    }

    #[test]
    fn test_group_by_same_order() {
        

        let mut df = df!(
            "field_1" => &[10,      20,     30,     40,   41,    50,     60,     70,     80,     90], 
            "field_2" => &[1,       1,      1,      1,    3,     2,      2,      2,      2,      2],
            "field_3" => &["a",     "b",    "c",    "d",  "dd",  "e",   "f",    "g",    "h",    "ii"]
        ).unwrap();

        /*
        _____|_0-20_|_20-40_|_40-60_|_60-80_|_80-100_|
         0-2 |  a      bc      d
         2-4 |                dd e    fg      h ii
         4-6 |
         6-8 |
         8-10|
        
         */
        let df = group_by(&df, "field_2", 2,"field_1", 20,[col("field_3").alias("field_3_value")]).expect("df");
        
        debug!("df={:?}", df);
       
        assert_eq!(
            *df.column("field_3_value").expect("field_3_value"),
            Series::new("field_3_value", [
                List(Series::new("", ["a"])),
                List(Series::new("", ["b", "c"])),
                List(Series::new("", ["d"])),
                List(Series::new("", ["dd", "e"])),
                List(Series::new("", ["f", "g"])),
                List(Series::new("", ["h", "ii"])),
            ])
        );
    }

    #[test]
    fn test_group_by_same_order_level_1() {


        let mut df = df!(
            "field_1" => &[10,      20,     30,     40,   41,    50,     60,     70,     80,     90], 
            "field_2" => &[1,       1,      1,      1,    3,     2,      2,      2,      2,      2],
            "field_3" => &["a",     "b",    "c",    "d",  "dd",  "e",   "f",    "g",    "h",    "ii"]
        ).unwrap();

        /*
        _____|_0-20_|_20-40_|_40-60_|_60-80_|_80-100_|
         0-2 |  a      bc      d
         2-4 |                dd e    fg      h ii
         4-6 |
         6-8 |
         8-10|
         */

        let field_1_col = df.column("field_1").unwrap();
        let field_2_col = df.column("field_2").unwrap();
        df = df.clone().lazy().filter(
            (col("field_1").gt_eq(40).and(col("field_1").lt(60).and(col("field_2").gt_eq(1).and(col("field_2").lt(3)))))
        ).collect().unwrap();

        debug!("df filtered ={:?}", df);
        
        let df = group_by(&df, "field_2", 2,"field_1", 20,[col("field_3").alias("field_3_value")]).expect("df");

        

        assert_eq!(
            *df.column("field_3_value").expect("field_3_value"),
            Series::new("field_3_value", [
                List(Series::new("", ["d"])),
                List(Series::new("", ["e"])),
            ])
        );
    }

    #[test]
    fn test_group_by_same_order_sum() {


        let mut df = df!(
            "field_1" => &[10,      20,     30,     40,   41,    50,     60,     70,     80,     90], 
            "field_2" => &[1,       1,      1,      1,    3,     2,      2,      2,      2,      2],
            "field_3" => &["a",     "b",    "c",    "d",  "dd",  "e",   "f",    "g",    "h",    "ii"],
            "field_4" => &[0.1,     0.1,    0.1,    0.1,  0.1,   0.1,   0.1,    0.1,    0.1,    0.1]
        ).unwrap();

        /*
        _____|_0-20_|_20-40_|_40-60_|_60-80_|_80-100_|
         0-2 |  a      bc      d
         2-4 |                dd e    fg      h ii
         4-6 |
         6-8 |
         8-10|
        
         */
        let mut df = group_by(
            &df, "field_2",  2,"field_1",  20,
            [
                col("field_3").count().alias("field_3_count"),
                col("field_4").sum().alias("field_4_sum")
            ]
        ).expect("df");

        debug!("df={:?}", df);

        assert_eq!(
            *df.column("field_3_count").expect("field_3_count"),
            Series::new("field_3_count", [
                1u32,
                2u32,
                1u32,
                2u32,
                2u32,
                2u32,
            ])
        );

        assert_eq!(
            *df.column("field_4_sum").expect("field_3_count"),
            Series::new("field_4_sum", [
                0.1,
                0.2,
                0.1,
                0.2,
                0.2,
                0.2,
            ])
        );

        df.as_single_chunk_par();
        let mut iters = df.columns(["primary_field_group", "secondary_group_field", "field_3_count", "field_4_sum"]).unwrap()
            .iter().map(|s| {
            debug!("s={:?}", s);
            s.iter()
        }).collect::<Vec<_>>();

        let mut primary_group:Option<i32> = None; 
        let mut secondary_group:Option<i32> = None; 

        
        for row in 0..df.height() {
            // for iter in &mut iters {
            //     let value = iter.next().expect("should have as many iterations as rows");
            //     debug!("value={:?}", value);
            // }

            let row_vec = iters.iter_mut().map(|i|i.next().unwrap()).collect::<Vec<_>>();
            

            debug!("row={:?}", row_vec);
        }

    }


    #[test]
    fn test_group_by_second_field_() {


        let mut df = df!(
            "field_1" => &[10,      20,     30,     40,   41,    50,     60,     70,     80,     90], 
            "field_2" => &[1,       2,      1,      2,    1,     2,      1,      2,      1,      2],
            "field_3" => &["a",     "b",    "c",    "d",  "dd",  "e",   "f",    "g",    "h",    "ii"]
        ).unwrap();

        /*
        _____|_0-20_|_20-40_|_40-60_|_60-80_|_80-100_|
         0-2 |  a      b      dd      f        h
         2-4 |         c      d e     g       ii
         4-6 |
         6-8 |
         8-10|
        
         */
        let df = group_by(&df, "field_2", 2, "field_1",  20,[col("field_3").alias("field_3_value")]).expect("df");

        debug!("df={:?}", df);

        assert_eq!(
            *df.column("field_3_value").expect("field_3_value"),
            Series::new("field_3_value", [
                List(Series::new("", ["a"])),
                List(Series::new("", ["c"])),
                List(Series::new("", ["dd"])),
                List(Series::new("", ["f"])),
                List(Series::new("", ["h"])),
                List(Series::new("", ["b"])),
                List(Series::new("", ["d", "e"])),
                List(Series::new("", ["g"])),
                List(Series::new("", ["ii"])),
            ])
        );


    }


    #[test]
    fn test_simp(){
        let df = df!(
        "field_1" => &[1,       1,      2,      2,    2],
        "field_2" => &["a",     "b",    "c",    "d",  "e"]
    ).unwrap();

        let group_by = df.groupby(["field_1"]).unwrap();
        let by = group_by.select(["field_2"]);
        let df = by.groups().unwrap();

        debug!("parquet_scan: df={:?}", df);
        

    }

}