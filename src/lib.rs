//! # Oracle Blueprint
//! Component aggregating Oracle data and processes it into data usable by the Proxy Component.

use scrypto::prelude::*;

#[derive(ScryptoSbor, Clone)]
pub struct PriceMessage {
    pub market_id: String,
    pub price: Decimal,
    pub nonce: u64,
    pub created_at: u64,
}

#[blueprint]
mod oracle {
    enable_method_auth! {
        methods {
            get_prices => PUBLIC;
            set_price => PUBLIC;
            add_pair => restrict_to: [OWNER];
        }
    }

    const LSU_POOL: Global<LsuPool> = global_component!(
        LsuPool,
        "component_rdx1cppy08xgra5tv5melsjtj79c0ngvrlmzl8hhs7vwtzknp9xxs63mfp"
    );

    extern_blueprint! {
        //"package_sim1pkgxxxxxxxxxpackgexxxxxxxxx000726633226xxxxxxxxxlk8hc9", //simulator package, uncomment to run tests
        //"package_tdx_2_1phrthm8neequrhdg8jxvvwd8xazccuaa8u3ufyemysade0ckv88an2", //stokenet morpher package
        "package_rdx1pka62r6e9754snp524ng3kfrkxma6qdxhzw86j7ka5nnl9m75nagmp", //mainnet morpher package
        MorpherOracle {
            fn check_price_input(&self, message: String, signature: String) -> PriceMessage;
        }

        // oracle address for stokenet: component_tdx_2_1cpt6kp3mqkds5uy858mqedwfglhsw25lhey59ev45ayce4yfsghf90
        // oracle address for mainnet: component_rdx1cpuqchky58ualnunh485cqne7p6dkepuwq0us2t5n89mz32k6pfppz
    }

    extern_blueprint! {
        "package_rdx1pkfrtmv980h85c9nvhxa7c9y0z4vxzt25c3gdzywz5l52g5t0hdeey", //mainnet lsu pool
        LsuPool {
            fn get_dex_valuation_xrd(&self) -> Decimal;
            fn get_liquidity_token_total_supply(&self) -> Decimal;
        }

        // lsu lp address: resource_rdx1thksg5ng70g9mmy9ne7wz0sc7auzrrwy7fmgcxzel2gvp8pj0xxfmf
    }

    struct Oracle {
        prices: Vec<(ResourceAddress, Decimal, u64, String)>,
        oracle_address: ComponentAddress,
    }

    impl Oracle {
        pub fn instantiate_oracle(
            owner_role: OwnerRole,
            oracle_address: ComponentAddress,
            dapp_def_address: GlobalAddress,
            resource_address_lsulp: ResourceAddress,
        ) -> Global<Oracle> {
            let prices: Vec<(ResourceAddress, Decimal, u64, String)> = vec![
                (
                    XRD,
                    dec!("0.015"),
                    Clock::current_time_rounded_to_seconds().seconds_since_unix_epoch as u64,
                    "GATEIO:XRD_USDT".to_string(),
                ),
                (
                    resource_address_lsulp,
                    dec!("0.015"),
                    Clock::current_time_rounded_to_seconds().seconds_since_unix_epoch as u64,
                    "LSULP".to_string(),
                ),
            ];

            Self {
                prices,
                oracle_address,
            }
            .instantiate()
            .prepare_to_globalize(owner_role)
            .metadata(metadata! {
                init {
                    "name" => "STAB Oracle".to_string(), updatable;
                    "description" => "An oracle used to keep track of collateral prices for STAB".to_string(), updatable;
                    "info_url" => Url::of("https://ilikeitstable.com"), updatable;
                    "dapp_definition" => dapp_def_address, updatable;
                }
            })
            .globalize()
        }

        pub fn get_prices(&mut self) -> Vec<(ResourceAddress, Decimal, u64, String)> {
            self.prices.clone()
        }

        pub fn set_price(&mut self, message: String, signature: String) {
            let morpher_oracle = Global::<MorpherOracle>::from(self.oracle_address);
            let price_message = morpher_oracle.check_price_input(message, signature);
            let mut price_is_xrd: bool = false;

            for price in self.prices.iter_mut() {
                if price.3 == price_message.market_id {
                    assert!(price_message.created_at > price.2, "Price is too old");
                    price.1 = price_message.price;
                    price.2 = price_message.created_at;

                    if price.0 == XRD {
                        price_is_xrd = true;
                    }
                }
            }
            if price_is_xrd {
                let lsu_multiplier: Decimal = //dec!("1.1"); (uncomment for Stokenet)
                LSU_POOL.get_dex_valuation_xrd() / LSU_POOL.get_liquidity_token_total_supply();
                self.prices[1].1 = price_message.price * lsu_multiplier;
                self.prices[1].2 = price_message.created_at;
            }
        }

        pub fn add_pair(
            &mut self,
            resource_address: ResourceAddress,
            market_id: String,
            starting_price: Decimal,
        ) {
            self.prices.push((
                resource_address,
                starting_price,
                Clock::current_time_rounded_to_seconds().seconds_since_unix_epoch as u64,
                market_id,
            ));
        }
    }
}
