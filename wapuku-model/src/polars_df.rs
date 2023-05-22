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
use polars::prelude::StartBy::WindowBound;
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

    fn build_grid(&self, property_x: PropertyRange, property_y: PropertyRange, x_n: u8, y_n: u8, group_volume_property: &str) -> GroupsGrid {
        let property_x_name = property_x.property().name().as_str();
        let property_y_name = property_y.property().name().as_str();

        let properties_df = self.df.select([property_x_name, property_y_name]).unwrap();
        let min_df = properties_df.min();
        let max_df = properties_df.max();

        let mut min_x =  property_x.min().unwrap_or(min_df.column(property_x_name).unwrap().get(0).unwrap().try_extract::<f32>().unwrap() as i64) as f32;
        let max_x = property_x.max().unwrap_or(max_df.column(property_x_name).unwrap().get(0).unwrap().try_extract::<f32>().unwrap() as i64) as f32;
        
        // if max_x - min_x + 1.0 < x_n as f32 {
        //     //group_1
        //     todo!()
        // }

        // if max_x - min_x < x_n as f32 {
        //     min_x = max_x - x_n as f32
        // }
        debug!("min_df={:?} max_df={:?}", min_df, max_df);

       
        let (property_x_step, d_x) = ( (( (max_x - min_x) / x_n as f32).ceil()) as i64, 0); //if max_x - min_x <= x_n as f32 {(1, 0)} else {((((max_x - min_x) * 1.1)/(x_n as f32)).ceil() as i64, ((max_x - min_x) * 0.05 -1.).abs().ceil() as i64)};
        // let (property_x_step, d_x) = ( (( (max_x - min_x) * 1.1 / x_n as f32).ceil()) as i64, (((max_x - min_x) * 1.1 - (max_x - min_x))/2.) as i64); //if max_x - min_x <= x_n as f32 {(1, 0)} else {((((max_x - min_x) * 1.1)/(x_n as f32)).ceil() as i64, ((max_x - min_x) * 0.05 -1.).abs().ceil() as i64)};
        // let property_x_step = if property_x_step == 0 {1} else {property_x_step};
        // let d_x = if d_x <= property_x_step {0} else {d_x}; //if first group completely hidden it's not returned from groupby_dynamic

        let mut min_y =  property_y.min().unwrap_or(min_df.column(property_y_name).unwrap().get(0).unwrap().try_extract::<f32>().unwrap() as i64) as f32;
        let max_y = property_y.max().unwrap_or(max_df.column(property_y_name).unwrap().get(0).unwrap().try_extract::<f32>().unwrap() as i64) as f32;

        // if max_y - min_y + 1.0 < y_n as f32 {
        //     //group_1
        //     todo!()
        // }

        // if max_y - min_y < y_n as f32 {
        //     min_y = max_y - y_n as f32
        // }
        
        // let d_y = (max_y * 0.05 -1.).abs().ceil() as i64;
        // let property_y_step = if max_y - min_y < y_n as f32 {1} else {((max_y * 1.1)/(y_n as f32)).ceil() as i64};
        // let (property_y_step, d_y) = ((((max_y - min_y) * 1.1 / y_n as f32).ceil()) as i64, (((max_y - min_y) * 1.1 - (max_y - min_y))/2.) as i64); //if max_y - min_y <= y_n as f32 {(1, 0)} else {((((max_y  - min_y)* 1.1)/(y_n as f32)).ceil() as i64, ((max_y  - min_y) * 0.05 -1.).abs().ceil() as i64)};
        let (property_y_step, d_y) = ((((max_y - min_y)  / y_n as f32).ceil()) as i64, 0); //if max_y - min_y <= y_n as f32 {(1, 0)} else {((((max_y  - min_y)* 1.1)/(y_n as f32)).ceil() as i64, ((max_y  - min_y) * 0.05 -1.).abs().ceil() as i64)};
        // let property_y_step = if property_y_step == 0 {1} else {property_y_step};
        // let d_y = if d_y <= property_y_step {0} else {d_y};
        
        debug!("min/max_x={:?}, min/max_y={:?} property_x_step={:?}, property_y_step={:?} d_x={}, d_y={}", (min_x, max_x), (min_y, max_y), property_x_step, property_y_step, d_x, d_y);

        let mut df = group_by_2(&self.df,
                                property_x_name, property_x_step,
                                property_y_name, property_y_step,
                                [
              col(group_volume_property).count().alias("volume"),
          ], min_x as i64 - property_x_step, min_y as i64 - property_y_step
        ).unwrap();
        
        debug!("df={:?}", df);

        // https://stackoverflow.com/questions/72440403/iterate-over-rows-polars-rust df is small here, should be ok
        df.as_single_chunk_par();
        let mut iters = df.columns(["primary_field_group", "secondary_group_field", "volume"]).unwrap()
            .iter().map(|s| {
            // debug!("s={:?}", s);
            s.iter()
        }).collect::<Vec<_>>();

        let mut primary_group:Option<i32> = None;
        let mut secondary_group:Option<i32> = None;

        let mut y_hash:HashMap<i64, HashMap<i64, Vec<AnyValue>>> = HashMap::new();

        let mut data_vec : Vec<Vec<Option<Box<dyn DataGroup>>>> = (0..y_n).map(|y| (0..x_n).map(|x| None).collect()).collect();

        //     (0..y_n).iter().for_each(|y|
        //                      (0..x_n).for_each(|x| {
        //                          let x_0 = min_x as i64 + x as i64 * property_x_step - d_x;
        //                          let x_1 = min_x as i64 + x as i64 * property_x_step + property_x_step - d_x;
        //                          let y_0 = min_y as i64 + y as i64 * property_y_step - d_y;
        //                          let y_1 = min_y as i64 + y as i64 * property_y_step + property_y_step - d_y;
        // 
        //                          // let mut row_vec = iters.iter_mut().map(|i|i.next().unwrap()).collect::<Vec<_>>();
        // 
        // 
        //                          let count = df.clone().lazy()
        //                              .filter(
        //                                  col("primary_field_group").gt_eq(x_0).and(col("primary_field_group").lt(x_1)
        //                                      .and(col("secondary_group_field").gt_eq(y_0).and(col("secondary_group_field").lt(y_1)))))
        //                              .select([col("volume")]).collect().unwrap();
        //                      });
        // });
        (0..y_n as usize).for_each(|y|{
            (0..x_n as usize).for_each(|x| {
                let x_0 = min_x as i64 + x as i64 * property_x_step ;
                 let x_1 = min_x as i64 + x as i64 * property_x_step + property_x_step;
                 let y_0 = min_y as i64 + y as i64 * property_y_step;
                 let y_1 = min_y as i64 + y as i64 * property_y_step + property_y_step;

                let count = df.clone().lazy()
                     .filter(
                         col("primary_field_group").gt_eq(x_0).and(col("primary_field_group").lt(x_1)
                             .and(col("secondary_group_field").gt_eq(y_0).and(col("secondary_group_field").lt(y_1)))))
                     .select([col("volume")]).collect().unwrap();
                let v = count.get(0).and_then(|v| v.get(0).map(|v| v.try_extract::<u32>().unwrap())).unwrap_or(0) as usize;
                
                
                debug!("data_vec: (x, y)={:?}, (x_0, x_1)={:?}, (y_0, y_1)={:?}, v={:?}", (x, y), (x_0, x_1), (y_0, y_1), v);
                
                if v > 0 {
                    let group_box = Box::<dyn DataGroup>::from(Box::new(SimpleDataGroup::new(
                        v,
                        // row_vec.get(2).map(|a|a.try_extract::<usize>().unwrap()).unwrap_or(0),
                        vec![],
                        DataBounds::XY(property_x.to_range(Some(x_0), Some(x_1)), property_y.to_range(Some(y_0), Some(y_1))),
                    )));

                    data_vec[y][x].replace(group_box);
                }
                
            });
        });
/*        for row in 0..df.height() {
            // for iter in &mut iters {
            //     let value = iter.next().expect("should have as many iterations as rows");
            //     debug!("value={:?}", value);
            // }
        
            let mut row_vec = iters.iter_mut().map(|i|i.next().unwrap()).collect::<Vec<_>>();
        
            //secondary property - Y - row_vec[1]
            //primary property - X  -row_vec[0]
            //GroupsGrid is row-major
            let mut x_hashmap = y_hash.entry(row_vec[1].try_extract::<i64>().unwrap()).or_insert(HashMap::new());
            x_hashmap.entry(row_vec[0].try_extract::<i64>().unwrap()).or_insert(row_vec[2..].to_vec());

            let x_0 = row_vec[0].try_extract::<i64>().expect("primary") - min_x as i64;
            let x_1 = x_0 + property_x_step;
            let grid_x = x_0 /property_x_step as i64; //if x_0 /property_x_step as i64 <= 0 {0} else {x_0 /property_x_step as i64};
            
            let y_0 = row_vec[1].try_extract::<i64>().expect("secondary") - min_y as i64;
            let grid_y = y_0 /property_y_step; //if y_0 /property_y_step as i64 <= 0 {0} else {y_0 /property_y_step as i64};
            let y_1 = y_0 + property_y_step;
            
            

            let group_box = Box::<dyn DataGroup>::from(Box::new(SimpleDataGroup::new(
                1,
                // row_vec.get(2).map(|a|a.try_extract::<usize>().unwrap()).unwrap_or(0),
                vec![],
                DataBounds::XY(property_x.to_range(Some(x_0), Some(x_1)), property_y.to_range(Some(y_0), Some(y_1))),
            )));
        // }

            debug!("adding x_0={:?}, grid_x={:?},  y_0={:?}, grid_y={:?} row_vec[2..]={:?}", x_0, grid_x as usize,  y_0, grid_y as usize, row_vec[2..].to_vec());
        
            data_vec[grid_y as usize][grid_x as usize].replace(group_box);

            
            
        }
*/
        // debug!("y_hash={:?}", y_hash);

        GroupsGrid::new(
            property_x.property().clone_to_box(),
            property_y.property().clone_to_box(),
            data_vec
            // (0..y_n).map(|y|
            //     (0..x_n).map(|x| {
            //         let x_0 = min_x as i64 + x as i64 * property_x_step - d_x;
            //         let x_1 = min_x as i64 + x as i64 * property_x_step + property_x_step - d_x;
            //         let y_0 = min_y as i64 + y as i64 * property_y_step - d_y;
            //         let y_1 = min_y as i64 + y as i64 * property_y_step + property_y_step - d_y;
            // 
            //         // let mut row_vec = iters.iter_mut().map(|i|i.next().unwrap()).collect::<Vec<_>>();
            // 
            // 
            //         let count = df.clone().lazy()
            //             .filter(
            //                 col("primary_field_group").gt_eq(x_0).and(col("primary_field_group").lt(x_1)
            //                     .and(col("secondary_group_field").gt_eq(y_0).and(col("secondary_group_field").lt(y_1)))))
            //             .select([col("volume")]).collect().unwrap();
            // 
            //         let v = count.get(0).and_then(|v| v.get(0).map(|v| v.try_extract::<u32>().unwrap())).unwrap_or(0) as usize;
            //         debug!("x={:?}, y={:?} count={:?}", (x_0, x_1), (y_0, y_1), v);
            // 
            //         // let v = y_hash.get(&y_0).and_then(|h| h.get(&x_0)).and_then(|v| v.get(0).map(|a| a.try_extract::<usize>().unwrap())).unwrap_or(0);
            // 
            //         debug!("x={:?}, y={:?}, x_0={:?}, y_0={:?}", x, y, x_0, y_0);
            // 
            //         Some(Box::<dyn DataGroup>::from(Box::new(SimpleDataGroup::new(
            //             v,
            //             // row_vec.get(2).map(|a|a.try_extract::<usize>().unwrap()).unwrap_or(0),
            //             vec![],
            //             DataBounds::XY(property_x.to_range(Some(x_0), Some(x_1)), property_y.to_range(Some(y_0), Some(y_1))),
            //         ))))
            //     }
            //     ).collect::<Vec<Option<Box<dyn DataGroup>>>>()
            // 
            // ).collect::<Vec<Vec<Option<Box<dyn DataGroup>>>>>()

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



pub(crate) fn group_by_2<E: AsRef<[Expr]>>(df:&DataFrame, primary_group_by_field: &str, primary_step: i64, secondary_group_by_field: &str, secondary_step: i64, aggregations: E, primary_offset: i64, secondary_offset: i64) -> WapukuResult<DataFrame> {
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
            offset: Duration::new(primary_offset),
            truncate: false,
            include_boundaries: true,
            closed_window: ClosedWindow::Left,
            start_by: WindowBound,
        }
    ).agg([
        col(primary_field_group).alias(primary_field_value)
    ])/*.with_row_count("primary_index", None)*/
        .explode([primary_field_value]).collect()?;//primary_field_in_group not used, for debug


    debug!("primary_field_grouped_and_expanded={:?}", primary_field_grouped_and_expanded);
    let mut df = df.sort([primary_group_by_field], false)?;
    
    let df = df
        .with_column(primary_field_grouped_and_expanded.column(primary_field_group).unwrap().clone())?
        // .with_column(primary_field_grouped_and_expanded.column("primary_index").unwrap().clone())?
        // .with_column({
        //     let mut series = primary_field_grouped_and_expanded.column("_lower_boundary").unwrap().clone();
        //     series.rename("primary_lower_boundary");
        //     series
        // })?
    ;
    let mut df = df.sort([secondary_group_by_field], false)?;
    
    // let mut df = df.left_join(&primary_field_grouped_and_expanded, [primary_group_by_field], ["primary_field_value"] )?;

    
    debug!("df={:?}", df);
    
    let mut df = df.clone()
        .lazy()
        .groupby_dynamic(
            [col(primary_field_group)],
            DynamicGroupOptions {
                index_column: secondary_group_by_field.into(),
                every: Duration::new(secondary_step),
                period: Duration::new(secondary_step),
                offset: Duration::new(secondary_offset),
                truncate: false,
                include_boundaries: true,
                closed_window: ClosedWindow::Left,
                start_by: WindowBound,
         }
        )
        .agg(aggregations)
        // .agg([
        //     col("property_3").count().alias("volume")
        // ])
        .collect()?;
    
    df.rename(secondary_group_by_field, "secondary_group_field");
    
    debug!("df grouped={:?}", df);

    Ok(df)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::time::Instant;
    use log::debug;
    use polars::datatypes::AnyValue::List;
    use polars::df;
    use polars::prelude::*;
    use crate::model::{Data, DataGroup, GroupsGrid, Property, PropertyRange, VecY};

    use crate::polars_df::{group_by_2, PolarsData};
    use crate::tests::init_log;

    #[ctor::ctor]
    fn init() {
        std::env::set_var("POLARS_FMT_MAX_ROWS", "100");
        std::env::set_var("FMT_MAX_COLS", "1000");
        
        init_log();
    }

    #[test]
    fn test_polars_data_1x3x3(){
        /**
         property_1     1   2   3    - X
         property_2
            1           11  12  13 
            2           21  22  23
            3           31  32  33

            Y
         
        **/

        let mut df = df!(
            "property_1" => &[1,   2,   3,   1,   2,   3,   1,   2,   3,], 
            "property_2" => &[10,  10,  10,  20,  20,  20,  30,  30,  30,],
            "property_3" => &[11,  12,  13,  21,  22,  23,  31,  32,  33,] 
        ).unwrap();
        

        debug!("df: {:?}", df);


        let mut grid = x_property_1_y_property_2_to_3_x_3_data(df, (Some(1i64), Some(4i64)), (Some(10i64), Some(31i64)));
        let data = grid.data();

        debug!("data: {:?}", data);

        
        assert_eq!(data[0][0].as_ref().unwrap().volume(), 1);
        assert_eq!(data[0][1].as_ref().unwrap().volume(), 1);
        assert_eq!(data[0][2].as_ref().unwrap().volume(), 1);

        assert_eq!(data[1][0].as_ref().unwrap().volume(), 1);
        assert_eq!(data[1][1].as_ref().unwrap().volume(), 1);
        assert_eq!(data[1][2].as_ref().unwrap().volume(), 1);

        assert_eq!(data[2][0].as_ref().unwrap().volume(), 1);
        assert_eq!(data[2][1].as_ref().unwrap().volume(), 1);
        assert_eq!(data[2][2].as_ref().unwrap().volume(), 1);

        // debug!(" data[0][0].volume()={:?}",  data[0][0].volume());
        // debug!(" data[0][1].volume()={:?}",  data[0][1].volume());
        // debug!(" data[0][2].volume()={:?}",  data[0][2].volume());
        // 
        // debug!(" data[1][0].volume()={:?}",  data[1][0].volume());
        // debug!(" data[1][1].volume()={:?}",  data[1][1].volume());
        // debug!(" data[1][2].volume()={:?}",  data[1][2].volume());
        // 
        // debug!(" data[2][0].volume()={:?}",  data[2][0].volume());
        // debug!(" data[2][1].volume()={:?}",  data[2][1].volume());
        // debug!(" data[2][2].volume()={:?}",  data[2][2].volume());
    }


    #[test]
    fn test_polars_data_1x2x3(){
        /**
         property_1      2   3    - X
         property_2
            1            12  13 
            2            22  23
            3            32  33

            Y
         
        **/

        let mut df = df!(
            "property_1" => &[2,   3,   2,   3,   2,   3,], 
            "property_2" => &[10,   20,   30,   10,   20,   30,],
            "property_3" => &[12,  13,  22,  23,  32,  33,] 
        ).unwrap();


        debug!("df: {:?}", df);

        let mut grid = x_property_1_y_property_2_to_3_x_3_data(df, (Some(1i64), Some(4i64)), (Some(10i64), Some(31i64)));
        // let data = grid.data();
        
        debug!("grid={:?}", grid);

        // debug!(" data[0][0].volume()={:?}",  grid.group_at(0, 0).map(|g|g.volume()));
        // debug!(" data[0][1].volume()={:?}",  data[0][1].as_ref().unwrap().volume());
        // debug!(" data[0][2].volume()={:?}",  data[0][2].as_ref().unwrap().volume());
        // 
        // debug!(" data[1][0].volume()={:?}",  data[1][0].as_ref().unwrap().volume());
        // debug!(" data[1][1].volume()={:?}",  data[1][1].as_ref().unwrap().volume());
        // debug!(" data[1][2].volume()={:?}",  data[1][2].as_ref().unwrap().volume());
        // 
        // debug!(" data[2][0].volume()={:?}",  data[2][0].as_ref().unwrap().volume());
        // debug!(" data[2][1].volume()={:?}",  data[2][1].as_ref().unwrap().volume());
        // debug!(" data[2][2].volume()={:?}",  data[2][2].as_ref().unwrap().volume());
        
        assert_eq!(grid.group_at(0, 0).is_none(), true);
        assert_eq!(grid.group_at(1, 0).unwrap().volume(), 1);
        assert_eq!(grid.group_at(2, 0).unwrap().volume(), 1);

        assert_eq!(grid.group_at(0, 1).is_none(), true);
        assert_eq!(grid.group_at(1, 1).unwrap().volume(), 1);
        assert_eq!(grid.group_at(2, 1).unwrap().volume(), 1);

        assert_eq!(grid.group_at(0, 2).is_none(), true);
        assert_eq!(grid.group_at(1, 2).unwrap().volume(), 1);
        assert_eq!(grid.group_at(2, 2).unwrap().volume(), 1);

    }

    #[test]
    fn test_polars_data_1x3x2(){
        /**
         property_1     1   2   3    - X
         property_2

            2           21  22  23
            3           31  32  33

            Y
        **/

        let mut df = df!(
            "property_1" => &[1,   2,   3,   1,   2,   3,], 
            "property_2" => &[20,   20,   20,   30,   30,   30,],
            "property_3" => &[21,  22,  23,  31,  32,  33,] 
        ).unwrap();


        debug!("df: {:?}", df);

        let mut grid = x_property_1_y_property_2_to_3_x_3_data(df, (Some(1i64), Some(4i64)), (Some(10i64), Some(31i64)));
        // let data = gird.data();


        assert_eq!(grid.group_at(0, 0).is_none(), true);
        assert_eq!(grid.group_at(0, 1).unwrap().volume(), 1);
        assert_eq!(grid.group_at(0, 2).unwrap().volume(), 1);

        assert_eq!(grid.group_at(1, 0).is_none(), true);
        assert_eq!(grid.group_at(1, 1).unwrap().volume(), 1);
        assert_eq!(grid.group_at(1, 2).unwrap().volume(), 1);
 
        assert_eq!(grid.group_at(2, 0).is_none(), true);
        assert_eq!(grid.group_at(2, 1).unwrap().volume(), 1);
        assert_eq!(grid.group_at(2, 2).unwrap().volume(), 1);

        // debug!(" data[0][0].volume()={:?}",  data[0][0].volume());
        // debug!(" data[0][1].volume()={:?}",  data[0][1].volume());
        // debug!(" data[0][2].volume()={:?}",  data[0][2].volume());
        // 
        // debug!(" data[1][0].volume()={:?}",  data[1][0].volume());
        // debug!(" data[1][1].volume()={:?}",  data[1][1].volume());
        // debug!(" data[1][2].volume()={:?}",  data[1][2].volume());
        // 
        // debug!(" data[2][0].volume()={:?}",  data[2][0].volume());
        // debug!(" data[2][1].volume()={:?}",  data[2][1].volume());
        // debug!(" data[2][2].volume()={:?}",  data[2][2].volume());
    }

    #[test]
    fn test_polars_data_1x4x3(){
        /**
         property_1     1   2   3   4 - X
         property_2
            1           11  12  13  14
            2           21  22  23  24
            3           31  32  33  24

            Y
         
        **/

        let mut df = df!(
            "property_1" => &[1,   2,   3,   4,   1,   2,   3,   4,  1,   2,   3,  4], 
            "property_2" => &[10,  10,  10,  10,  20,  20,  20,  20, 30,  30,  30, 30],
            "property_3" => &[11,  12,  13,  14,  21,  22,  23,  24, 31,  32,  33, 34] 
        ).unwrap();


        debug!("df: {:?}", df);

        let mut grid = x_property_1_y_property_2_to_3_x_3_data(df, (Some(0i64), Some(5i64)), (Some(0i64), Some(31i64)));
        // let data = grid.data();


        // debug!(" data[0][0].volume()={:?}",  data[0][0].as_ref().unwrap().volume());
        // debug!(" data[0][1].volume()={:?}",  data[0][1].as_ref().unwrap().volume());
        // debug!(" data[0][2].volume()={:?}",  data[0][2].as_ref().unwrap().volume());
        // 
        // debug!(" data[1][0].volume()={:?}",  data[1][0].as_ref().unwrap().volume());
        // debug!(" data[1][1].volume()={:?}",  data[1][1].as_ref().unwrap().volume());
        // debug!(" data[1][2].volume()={:?}",  data[1][2].as_ref().unwrap().volume());
        // 
        // debug!(" data[2][0].volume()={:?}",  data[2][0].as_ref().unwrap().volume());
        // debug!(" data[2][1].volume()={:?}",  data[2][1].as_ref().unwrap().volume());
        // debug!(" data[2][2].volume()={:?}",  data[2][2].as_ref().unwrap().volume());

        assert_eq!(grid.group_at(0, 0).unwrap().volume(), 1);
        assert_eq!(grid.group_at(1, 0).unwrap().volume(), 2);
        assert_eq!(grid.group_at(2, 0).unwrap().volume(), 1);

        assert_eq!(grid.group_at(0, 1).unwrap().volume(), 1);
        assert_eq!(grid.group_at(1, 1).unwrap().volume(), 2);
        assert_eq!(grid.group_at(2, 1).unwrap().volume(), 1);

        assert_eq!(grid.group_at(0, 2).unwrap().volume(), 1);
        assert_eq!(grid.group_at(1, 2).unwrap().volume(), 2);
        assert_eq!(grid.group_at(2, 2).unwrap().volume(), 1);

    }


    fn x_property_1_y_property_2_to_3_x_3_data(mut df: DataFrame, min_max_x: (Option<i64>, Option<i64>), min_max_y: (Option<i64>, Option<i64>)) -> GroupsGrid {
        let mut data = PolarsData::new(df);


        let all_properties: HashSet<&dyn Property> = data.all_properties();

        let (property_1, property_2, property_3) = {
            let mut all_properties_iter = all_properties.into_iter().collect::<Vec<&dyn Property>>();
            all_properties_iter.sort_by(|p1, p2| p1.name().cmp(p2.name()));

            (*all_properties_iter.get(0).expect("property_1"), *all_properties_iter.get(1).expect("property_2"), *all_properties_iter.get(2).expect("property_3"))
        };
        
         data.build_grid(
            PropertyRange::new(property_1, min_max_x.0, min_max_x.1),
            PropertyRange::new(property_2, min_max_y.0, min_max_y.1),
            3, 3, "property_3"
        )
        
    }


    /*#[test]
    fn test_polars_data(){
        
        let df = df!(
            "property_1" => &(0..10_000_000).into_iter().map(|i|i / 10).collect::<Vec<i64>>(), // 10 X 0, 10 X 1 ...
            "property_2" => &(0..10_000_000).into_iter().map(|i|i - (i/10)*10 ).collect::<Vec<i64>>(), // 
            "property_3" => &(0..10_000_000).into_iter().map(|i|i).collect::<Vec<i32>>(),
        ).unwrap();
        
        debug!("df: {:?}", df);
        let t_0 = Instant::now();
        
        let mut data = PolarsData::new(df);
        

        let all_properties:HashSet<&dyn Property> = data.all_properties();

        let (property_1, property_2, property_3) = {
            let mut all_properties_iter = all_properties.into_iter().collect::<Vec<&dyn Property>>();
            all_properties_iter.sort_by(|p1, p2|p1.name().cmp(p2.name()));

            (*all_properties_iter.get(0).expect("property_1"), *all_properties_iter.get(1).expect("property_2"), *all_properties_iter.get(2).expect("property_3"))
        };

        let mut data_grid = data.build_grid(
            PropertyRange::new (property_1,  None, None ),
            PropertyRange::new (property_2,  None, None ),
            3, 3, "property_3"
        );

        let data = data_grid.data();
        
        println!("done in {}", t_0.elapsed().as_millis());

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
    }*/

    #[test]
    fn test_build_grid_with_more_properties(){
        let mut df = df!(
            "property_1" => &[10,      20,     30,     40,    50,      60,      70,      80,      90, 100], 
            "property_2" => &[1,       1,      1,      1,     2,       2,       2,       2,       2,    2],
            "property_3" => &["a",     "b",    "c",    "d",   "e",     "f",     "g",     "h",     "i",  "j"],
            "property_4" => &[100,     200,    300,    400,   500,     600,     700,     800,     900,  1000],
            "property_5" => &[1000,    2000,   3000,   4000,  5000,    6000,    7000,    8000,    9000, 10000]
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

        let mut data_grid = polars_data.build_grid(
            PropertyRange::new (property_1,  None, None ),
            PropertyRange::new (property_2,  None, None ),
            3, 3, "property_3"
        );
        //TODO
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
        let df = group_by_2(
            &df, "field_2", 2,"field_1", 20,[col("field_3").alias("field_3_value")],
            0i64, 0i64
        ).expect("df");
        
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
    fn test_simp(){
        let df = df!(
        "field_1" => &[0, 1,   2,      3,      4],
        "field_2" => &["x", "a",   "b",    "c",    "d"],
        "field_3" => &[0, 10,  20,    30,    40]
    ).unwrap();

        // let group_by = df.groupby(["field_1"]).unwrap();
        // let by = group_by.select(["field_2"]);
        // let df = by.groups().unwrap();

        let df = df.clone().lazy().groupby_dynamic(
            [],
            DynamicGroupOptions {
                index_column: "field_1".into(),
                every: Duration::new(2),
                period: Duration::new(2),
                offset: Duration::new(-1),
                truncate: false,
                include_boundaries: true,
                closed_window: ClosedWindow::Left,
                start_by: StartBy::WindowBound,
            }

        )
            .agg([col("field_2").alias("field_2"), col("field_3")])
            .collect().unwrap();

        debug!("parquet_scan: df={:?}", df);


    }
    

}