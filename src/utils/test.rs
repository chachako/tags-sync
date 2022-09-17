#[cfg(test)]
#[macro_export]
macro_rules! test_fn {
    ($name:ident $block:block) => {
        paste::paste! {
            fn [<test_ $name _backing>]() -> anyhow::Result<()> {
                if let Ok(_) = pretty_env_logger::try_init() {
                    // Ignore
                }
                $block
                Ok(())
            }

            #[test]
            fn [<test_ $name>]() {
                [<test_ $name _backing>]().unwrap()
            }
        }
    };
}

#[cfg(test)]
#[macro_export]
macro_rules! test_async_fn {
    ($name:ident $block:block) => {
        paste::paste! {
            async fn [<test_ $name _backing>]() -> anyhow::Result<()> {
                if let Ok(_) = pretty_env_logger::try_init() {
                    // Ignore
                }
                $block
                Ok(())
            }

            #[tokio::test]
            async fn [<test_ $name>]() {
                [<test_ $name _backing>]().await.unwrap()
            }
        }
    };
}
