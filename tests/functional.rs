// Copyright 2020 Cognite AS

//! Functional test against an unleashed API server running locally.
//! Set environment variables as per config.rs to exercise this.
//!
//! Currently expects a feature called default with one strategy default
//! Additional features are ignored.

#[cfg(feature = "functional")]
mod tests {
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    use async_std::task;
    use enum_map::Enum;
    use futures_timer::Delay;
    use serde::{Deserialize, Serialize};

    use unleash_api_client::{client, config::EnvironmentConfig, http::HttpClient};

    #[cfg(not(any(feature = "surf", feature = "reqwest")))]
    compile_error!("Cannot run test suite without a client enabled");

    #[allow(non_camel_case_types)]
    #[derive(Debug, Deserialize, Serialize, Enum, Clone)]
    enum UserFeatures {
        default,
    }

    fn test_smoke_async<C>() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>
    where
        C: HttpClient + Default + 'static,
    {
        let _ = simple_logger::init();
        task::block_on(async {
            let config = EnvironmentConfig::from_env()?;
            let client = client::ClientBuilder::default()
                .interval(500)
                .into_client::<UserFeatures, C>(
                    &config.api_url,
                    &config.app_name,
                    &config.instance_id,
                    config.secret,
                )?;
            client.register().await?;
            futures::future::join(client.poll_for_updates(), async {
                // Ensure we have features
                Delay::new(Duration::from_millis(500)).await;
                assert!(client.is_enabled(UserFeatures::default, None, false));
                // Ensure the metrics get up-loaded
                Delay::new(Duration::from_millis(500)).await;
                client.stop_poll().await;
            })
            .await;
            println!("got metrics");
            Ok(())
        })
    }

    #[cfg(feature = "surf")]
    #[test]
    fn test_smoke_async_surf() {
        test_smoke_async::<surf::Client>().unwrap();
    }

    #[cfg(feature = "reqwest")]
    #[test]
    fn test_smoke_async_reqwest() {
        test_smoke_async::<reqwest::Client>().unwrap();
    }

    fn test_smoke_threaded<C>() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>
    where
        C: HttpClient + Default + 'static,
    {
        let _ = simple_logger::init();
        let config = EnvironmentConfig::from_env()?;
        let client = Arc::new(
            client::ClientBuilder::default()
                .interval(500)
                .into_client::<_, C>(
                    &config.api_url,
                    &config.app_name,
                    &config.instance_id,
                    config.secret,
                )?,
        );
        task::block_on(async {
            if let Err(e) = client.register().await {
                Err(e)
            } else {
                Ok(())
            }
        })?;
        // Spin off a polling thread
        let poll_handle = client.clone();
        // let poll_handle = think.clone();
        let handler = thread::spawn(move || {
            // thread code
            task::block_on(async {
                poll_handle.poll_for_updates().await;
            });
        });

        // Ensure we have features
        thread::sleep(Duration::from_millis(500));
        assert!(client.is_enabled(UserFeatures::default, None, false));
        // Ensure the metrics get up-loaded
        thread::sleep(Duration::from_millis(500));
        task::block_on(async {
            client.stop_poll().await;
        });
        handler.join().unwrap();
        println!("got metrics");
        Ok(())
    }

    #[cfg(feature = "surf")]
    #[test]
    fn test_smoke_threaded_surf() {
        test_smoke_threaded::<surf::Client>().unwrap();
    }

    #[cfg(feature = "reqwest")]
    #[test]
    fn test_smoke_threaded_reqwest() {
        test_smoke_threaded::<reqwest::Client>().unwrap();
    }
}
