// ── Currency commands ────────────────────────────────────────────────────────

pub struct CreateCurrency {
    pub code: String,
    pub name: String,
    pub symbol: String,
}
ddd_application::impl_command!(CreateCurrency, ());

pub struct UpdateCurrency {
    pub code: String,
    pub name: String,
    pub symbol: String,
}
ddd_application::impl_command!(UpdateCurrency, ());

pub struct DeleteCurrency { pub code: String }
ddd_application::impl_command!(DeleteCurrency, ());

pub struct ActivateCurrency { pub code: String }
ddd_application::impl_command!(ActivateCurrency, ());

pub struct DeactivateCurrency { pub code: String }
ddd_application::impl_command!(DeactivateCurrency, ());

// ── Country commands ─────────────────────────────────────────────────────────

pub struct CreateCountry {
    pub code: String,
    pub name: String,
    pub currency_code: String,
}
ddd_application::impl_command!(CreateCountry, ());

pub struct UpdateCountry {
    pub code: String,
    pub name: String,
    pub currency_code: String,
}
ddd_application::impl_command!(UpdateCountry, ());

pub struct DeleteCountry { pub code: String }
ddd_application::impl_command!(DeleteCountry, ());

pub struct ActivateCountry { pub code: String }
ddd_application::impl_command!(ActivateCountry, ());

pub struct DeactivateCountry { pub code: String }
ddd_application::impl_command!(DeactivateCountry, ());

// ── State commands ───────────────────────────────────────────────────────────

pub struct CreateState {
    pub code: String,
    pub name: String,
    pub country_code: String,
}
ddd_application::impl_command!(CreateState, ());

pub struct UpdateState {
    pub code: String,
    pub name: String,
}
ddd_application::impl_command!(UpdateState, ());

pub struct DeleteState { pub code: String }
ddd_application::impl_command!(DeleteState, ());

pub struct ActivateState { pub code: String }
ddd_application::impl_command!(ActivateState, ());

pub struct DeactivateState { pub code: String }
ddd_application::impl_command!(DeactivateState, ());

// ── City commands ────────────────────────────────────────────────────────────

pub struct CreateCity {
    pub code: String,
    pub name: String,
    pub state_code: String,
}
ddd_application::impl_command!(CreateCity, ());

pub struct UpdateCity {
    pub code: String,
    pub name: String,
}
ddd_application::impl_command!(UpdateCity, ());

pub struct DeleteCity { pub code: String }
ddd_application::impl_command!(DeleteCity, ());

pub struct ActivateCity { pub code: String }
ddd_application::impl_command!(ActivateCity, ());

pub struct DeactivateCity { pub code: String }
ddd_application::impl_command!(DeactivateCity, ());

// ── Pincode commands ─────────────────────────────────────────────────────────

pub struct CreatePincode {
    pub code: String,
    pub city_code: String,
    pub area_name: Option<String>,
}
ddd_application::impl_command!(CreatePincode, ());

pub struct UpdatePincode {
    pub code: String,
    pub area_name: Option<String>,
}
ddd_application::impl_command!(UpdatePincode, ());

pub struct DeletePincode { pub code: String }
ddd_application::impl_command!(DeletePincode, ());

pub struct ActivatePincode { pub code: String }
ddd_application::impl_command!(ActivatePincode, ());

pub struct DeactivatePincode { pub code: String }
ddd_application::impl_command!(DeactivatePincode, ());
