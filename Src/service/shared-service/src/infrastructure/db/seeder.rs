use std::sync::Arc;

use sea_orm::DatabaseConnection;
use tracing::info;

use crate::domain::entities::{City, Country, Currency, Pincode, State};
use crate::domain::ids::{CityCode, CountryCode, CurrencyCode, PincodeId, StateCode};
use crate::domain::repositories::{
    CityRepository, CountryRepository, CurrencyRepository, PincodeRepository, StateRepository,
};
use crate::infrastructure::db::repositories::{
    PgCityRepository, PgCountryRepository, PgCurrencyRepository, PgPincodeRepository,
    PgStateRepository,
};

pub async fn run_seeder(db: &Arc<DatabaseConnection>) {
    let currency_repo = PgCurrencyRepository(db.clone());
    let country_repo = PgCountryRepository(db.clone());
    let state_repo = PgStateRepository(db.clone());
    let city_repo = PgCityRepository(db.clone());
    let pincode_repo = PgPincodeRepository(db.clone());

    seed_currencies(&currency_repo).await;
    seed_countries(&country_repo).await;
    seed_states(&state_repo).await;
    seed_cities(&city_repo).await;
    seed_pincodes(&pincode_repo).await;

    info!("Seeder completed.");
}

async fn seed_currencies(repo: &PgCurrencyRepository) {
    let code = CurrencyCode::new("MYR");
    if repo.exists(&code).await.unwrap_or(false) {
        return;
    }
    if let Ok(c) = Currency::create(code, "Malaysian Ringgit".into(), "RM".into()) {
        if let Err(e) = repo.save(&c).await {
            tracing::warn!("Failed to seed MYR: {e}");
        } else {
            info!("Seeded currency MYR");
        }
    }
}

async fn seed_countries(repo: &PgCountryRepository) {
    let code = CountryCode::new("MY");
    if repo.exists(&code).await.unwrap_or(false) {
        return;
    }
    if let Ok(c) = Country::create(code, "Malaysia".into(), CurrencyCode::new("MYR")) {
        if let Err(e) = repo.save(&c).await {
            tracing::warn!("Failed to seed MY: {e}");
        } else {
            info!("Seeded country MY");
        }
    }
}

async fn seed_states(repo: &PgStateRepository) {
    let states = vec![
        ("JHR", "Johor"),
        ("KDH", "Kedah"),
        ("KTN", "Kelantan"),
        ("KUL", "Kuala Lumpur"),
        ("LBN", "Labuan"),
        ("MLK", "Melaka"),
        ("NSN", "Negeri Sembilan"),
        ("PHG", "Pahang"),
        ("PNG", "Pulau Pinang"),
        ("PRK", "Perak"),
        ("PJY", "Putrajaya"),
        ("PLS", "Perlis"),
        ("SBH", "Sabah"),
        ("SGR", "Selangor"),
        ("SRW", "Sarawak"),
        ("TRG", "Terengganu"),
    ];
    for (code, name) in states {
        let id = StateCode::new(code);
        if repo.exists(&id).await.unwrap_or(false) {
            continue;
        }
        if let Ok(s) = State::create(id, name.into(), CountryCode::new("MY")) {
            if let Err(e) = repo.save(&s).await {
                tracing::warn!("Failed to seed state {code}: {e}");
            }
        }
    }
    info!("Seeded Malaysian states");
}

async fn seed_cities(repo: &PgCityRepository) {
    let cities: Vec<(&str, &str, &str)> = vec![
        ("JB", "Johor Bahru", "JHR"),
        ("MSAI", "Muar", "JHR"),
        ("KLU", "Kluang", "JHR"),
        ("ALS", "Alor Setar", "KDH"),
        ("SGL", "Sungai Golok", "KDH"),
        ("KBHRU", "Kota Bharu", "KTN"),
        ("PSR", "Pasir Mas", "KTN"),
        ("KL", "Kuala Lumpur City", "KUL"),
        ("KLCC", "KLCC", "KUL"),
        ("BKTT", "Bukit Tunku", "KUL"),
        ("LBWNT", "Labuan Town", "LBN"),
        ("MLKC", "Melaka City", "MLK"),
        ("AYER", "Ayer Keroh", "MLK"),
        ("SEREM", "Seremban", "NSN"),
        ("PORTD", "Port Dickson", "NSN"),
        ("KUNTA", "Kuantan", "PHG"),
        ("BENTG", "Bentong", "PHG"),
        ("PGCIT", "Georgetown", "PNG"),
        ("BTTW", "Butterworth", "PNG"),
        ("IPOH", "Ipoh", "PRK"),
        ("TGRH", "Taiping", "PRK"),
        ("PJYC", "Putrajaya City", "PJY"),
        ("KANGA", "Kangar", "PLS"),
        ("KKINY", "Kota Kinabalu", "SBH"),
        ("SDKN", "Sandakan", "SBH"),
        ("PJ", "Petaling Jaya", "SGR"),
        ("SHA", "Shah Alam", "SGR"),
        ("KLANG", "Klang", "SGR"),
        ("AMPNG", "Ampang", "SGR"),
        ("KCHING", "Kuching", "SRW"),
        ("MIRI", "Miri", "SRW"),
        ("KUALTG", "Kuala Terengganu", "TRG"),
        ("KEMAMAN", "Kemaman", "TRG"),
    ];
    for (code, name, state_code) in cities {
        let id = CityCode::new(code);
        if repo.exists(&id).await.unwrap_or(false) {
            continue;
        }
        if let Ok(c) = City::create(id, name.into(), StateCode::new(state_code)) {
            if let Err(e) = repo.save(&c).await {
                tracing::warn!("Failed to seed city {code}: {e}");
            }
        }
    }
    info!("Seeded Malaysian cities");
}

async fn seed_pincodes(repo: &PgPincodeRepository) {
    let pincodes: Vec<(&str, &str, Option<&str>)> = vec![
        ("50000", "KL", Some("Kuala Lumpur City Centre")),
        ("50088", "KL", Some("Parliament / Lake Garden")),
        ("50100", "KL", Some("Chow Kit")),
        ("50200", "KL", Some("Masjid India")),
        ("50300", "KL", Some("Kuala Lumpur")),
        ("50350", "KL", Some("Chow Kit Road")),
        ("50400", "KL", Some("Pudu")),
        ("50450", "KL", Some("Bukit Bintang")),
        ("50460", "KL", Some("Jalan Imbi")),
        ("50470", "KL", Some("Jalan Pahang")),
        ("50480", "KL", Some("Jalan Sultan Ismail")),
        ("50490", "KL", Some("Jalan Ampang")),
        ("50500", "KL", Some("Jalan Semarak")),
        ("50603", "KL", Some("Titiwangsa")),
        ("50604", "KL", Some("Sentul")),
        ("50614", "KL", Some("Kepong")),
        ("50620", "KL", Some("Wangsa Maju")),
        ("50670", "KL", Some("Taman Tun Dr Ismail")),
        ("50694", "KL", Some("Damansara Heights")),
        ("51200", "KL", Some("Mont Kiara")),
        ("53000", "KL", Some("Setapak")),
        ("54000", "KL", Some("Jinjang")),
        ("55100", "KLCC", Some("KLCC / Ampang Park")),
        ("55200", "KLCC", Some("Jalan Tun Razak")),
        ("47500", "PJ", Some("Petaling Jaya Section 17")),
        ("46000", "PJ", Some("Petaling Jaya Old Town")),
        ("46050", "PJ", Some("Section 51A")),
        ("46100", "PJ", Some("Section 16 / 17")),
        ("46150", "PJ", Some("Petaling Jaya New Town")),
        ("46200", "PJ", Some("Damansara")),
        ("47400", "PJ", Some("Subang Jaya")),
        ("47600", "PJ", Some("Subang Airport")),
        ("40000", "SHA", Some("Shah Alam City Centre")),
        ("40150", "SHA", Some("Section 7")),
        ("40160", "SHA", Some("Section 14")),
        ("80000", "JB", Some("Johor Bahru City Centre")),
        ("80100", "JB", Some("Johor Bahru")),
        ("80200", "JB", Some("Larkin")),
        ("80300", "JB", Some("Tampoi")),
        ("80350", "JB", Some("Skudai")),
        ("10000", "PGCIT", Some("Georgetown City Centre")),
        ("10050", "PGCIT", Some("Georgetown")),
        ("10100", "PGCIT", Some("Penang Hill")),
        ("10150", "PGCIT", Some("Air Itam")),
        ("10400", "PGCIT", Some("Georgetown North")),
        ("30000", "IPOH", Some("Ipoh City Centre")),
        ("30100", "IPOH", Some("Ipoh")),
        ("30200", "IPOH", Some("Buntong")),
        ("30300", "IPOH", Some("Pasir Puteh")),
        ("20000", "KUALTG", Some("Kuala Terengganu City")),
        ("20050", "KUALTG", Some("Kuala Terengganu")),
        ("70000", "SEREM", Some("Seremban City Centre")),
        ("70100", "SEREM", Some("Seremban")),
        ("70200", "SEREM", Some("Rasah")),
        ("15000", "KBHRU", Some("Kota Bharu City")),
        ("15050", "KBHRU", Some("Kota Bharu")),
        ("15100", "KBHRU", Some("Kubang Kerian")),
        ("25000", "KUNTA", Some("Kuantan City Centre")),
        ("25050", "KUNTA", Some("Kuantan")),
        ("25100", "KUNTA", Some("Indera Mahkota")),
        ("75000", "MLKC", Some("Melaka City Centre")),
        ("75100", "MLKC", Some("Melaka Town")),
        ("75200", "MLKC", Some("Bandar Hilir")),
        ("88000", "KKINY", Some("Kota Kinabalu City Centre")),
        ("88100", "KKINY", Some("Kota Kinabalu")),
        ("88300", "KKINY", Some("Likas")),
        ("93000", "KCHING", Some("Kuching City Centre")),
        ("93100", "KCHING", Some("Kuching")),
        ("93200", "KCHING", Some("Samarahan")),
        ("05000", "ALS", Some("Alor Setar City Centre")),
        ("05050", "ALS", Some("Alor Setar")),
        ("62000", "PJYC", Some("Putrajaya Core")),
        ("62100", "PJYC", Some("Putrajaya Precinct 1")),
        ("62150", "PJYC", Some("Putrajaya Precinct 2")),
        ("62200", "PJYC", Some("Putrajaya Precinct 3")),
        ("87000", "LBWNT", Some("Labuan Town")),
        ("87020", "LBWNT", Some("Labuan Financial District")),
        ("02000", "KANGA", Some("Kangar City")),
        ("02600", "KANGA", Some("Kangar")),
    ];
    for (code, city_code, area_name) in pincodes {
        let id = PincodeId::new(code);
        if repo.exists(&id).await.unwrap_or(false) {
            continue;
        }
        if let Ok(p) = Pincode::create(id, CityCode::new(city_code), area_name.map(Into::into)) {
            if let Err(e) = repo.save(&p).await {
                tracing::warn!("Failed to seed pincode {code}: {e}");
            }
        }
    }
    info!("Seeded Malaysian pincodes");
}
