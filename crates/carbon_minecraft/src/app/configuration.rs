use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Ord, PartialOrd, PartialEq, Eq)]
pub(crate) struct AppConfiguration {
    id: i32,
}


#[cfg(test)]
mod unit_tests {
    use crate::db;
    use crate::db::app_configuration;
    use crate::db::app_configuration::SetParam::SetId;
    use crate::db::app_configuration::WhereParam;
    use crate::db::read_filters::IntFilter;


    #[tokio::test]
    #[tracing_test::traced_test]
    async fn persistence_ok() {

        let client = db::new_client().await
            .expect("unable to build app_configuration client");

        let configuration = client
            .app_configuration()
            .create(vec![SetId(0)])
            .exec()
            .await
            .expect("unable to exec create query for app_configuration");

        let serialized_configuration = serde_json::to_string_pretty(&configuration)
            .expect("unable to serialize app_configuration");

        let count = client.app_configuration()
            .count(vec![WhereParam::Id(IntFilter::Equals(0))])
            .exec().await
            .expect("unable to select app_configuration");

    }
}