pub mod api;
pub mod app;
pub(crate) mod db;


#[cfg(test)]
mod e2e {
    use log::trace;
    use crate::db;
    use crate::db::app_configuration::SetParam::SetId;
    use crate::db::app_configuration::{UniqueWhereParam, WhereParam};
    use crate::db::read_filters::IntFilter;

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn persistence_ok() {
        trace!("trying to connect to db ");
        let client = db::new_client().await
            .expect("unable to build app_configuration client using db_url ");
        trace!("connected to db");

        let configuration = client
            .app_configuration()
            .upsert(UniqueWhereParam::IdEquals(101),vec![SetId(101)] , vec![SetId(101)])
            .exec()
            .await
            .expect("unable to exec create query for app_configuration");

        trace!("wrote correctly in db : {:#?}",configuration);

        let _serialized_configuration = serde_json::to_string_pretty(&configuration)
            .expect("unable to serialize app_configuration");

        let _count = client.app_configuration()
            .count(vec![WhereParam::Id(IntFilter::Equals(101))])
            .exec().await
            .expect("unable to select app_configuration");

        trace!("read correctly from db ");
    }
}