use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrgRole {
    Admin,
    Manager,
    Member,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectRole {
    Lead,
    Freelancer,
    Admin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectType {
    TimeAndMaterials,
    FixedFee,
    NonBillable,
    Retainer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryState {
    Open,
    Submitted,
    Approved,
    Invoiced,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BudgetKind {
    Hours,
    Money,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoundDir {
    Up,
    Down,
    Nearest,
}

// ── Display ───────────────────────────────────────────────────────────────────

macro_rules! impl_display {
    ($ty:ty, $($variant:path => $s:literal),+ $(,)?) => {
        impl std::fmt::Display for $ty {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let s = match self {
                    $($variant => $s,)+
                };
                f.write_str(s)
            }
        }
    };
}

impl_display!(OrgRole,
    OrgRole::Admin   => "admin",
    OrgRole::Manager => "manager",
    OrgRole::Member  => "member",
);

impl_display!(ProjectRole,
    ProjectRole::Lead       => "lead",
    ProjectRole::Freelancer => "freelancer",
    ProjectRole::Admin      => "admin",
);

impl_display!(ProjectType,
    ProjectType::TimeAndMaterials => "time_and_materials",
    ProjectType::FixedFee         => "fixed_fee",
    ProjectType::NonBillable      => "non_billable",
    ProjectType::Retainer         => "retainer",
);

impl_display!(EntryState,
    EntryState::Open      => "open",
    EntryState::Submitted => "submitted",
    EntryState::Approved  => "approved",
    EntryState::Invoiced  => "invoiced",
);

impl_display!(BudgetKind,
    BudgetKind::Hours => "hours",
    BudgetKind::Money => "money",
    BudgetKind::None  => "none",
);

impl_display!(RoundDir,
    RoundDir::Up      => "up",
    RoundDir::Down    => "down",
    RoundDir::Nearest => "nearest",
);

// ── FromStr ───────────────────────────────────────────────────────────────────

macro_rules! from_str {
    ($ty:ty, $($s:literal => $variant:expr),+ $(,)?) => {
        impl std::str::FromStr for $ty {
            type Err = String;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $($s => Ok($variant),)+
                    other => Err(format!("unknown {} value: {:?}", stringify!($ty), other)),
                }
            }
        }
    };
}

from_str!(OrgRole,
    "admin"   => OrgRole::Admin,
    "manager" => OrgRole::Manager,
    "member"  => OrgRole::Member,
);

from_str!(ProjectRole,
    "lead"       => ProjectRole::Lead,
    "freelancer" => ProjectRole::Freelancer,
    "admin"      => ProjectRole::Admin,
);

from_str!(ProjectType,
    "time_and_materials" => ProjectType::TimeAndMaterials,
    "fixed_fee"          => ProjectType::FixedFee,
    "non_billable"       => ProjectType::NonBillable,
    "retainer"           => ProjectType::Retainer,
);

from_str!(EntryState,
    "open"      => EntryState::Open,
    "submitted" => EntryState::Submitted,
    "approved"  => EntryState::Approved,
    "invoiced"  => EntryState::Invoiced,
);

from_str!(BudgetKind,
    "hours"  => BudgetKind::Hours,
    "money"  => BudgetKind::Money,
    "none"   => BudgetKind::None,
);

from_str!(RoundDir,
    "up"      => RoundDir::Up,
    "down"    => RoundDir::Down,
    "nearest" => RoundDir::Nearest,
);

// ── sqlx Postgres support ─────────────────────────────────────────────────────
//
// Each enum maps to its Postgres enum type. Decode/Encode go through &str so
// the implementations work with both `SELECT col` (returns the enum OID) and
// `SELECT col::text` (returns text), because Postgres sends both as text in the
// simple query protocol used by sqlx.

#[cfg(feature = "sqlx")]
mod sqlx_impls {
    use super::*;
    use sqlx::encode::IsNull;
    use sqlx::error::BoxDynError;
    use sqlx::postgres::{PgArgumentBuffer, PgTypeInfo, PgValueRef};

    macro_rules! pg_enum {
        ($ty:ty, $pg_name:literal, $($variant:path => $s:literal),+ $(,)?) => {
            impl sqlx::Type<sqlx::Postgres> for $ty {
                fn type_info() -> PgTypeInfo {
                    PgTypeInfo::with_name($pg_name)
                }
                fn compatible(ty: &PgTypeInfo) -> bool {
                    // Accept both the native enum OID and plain text
                    *ty == Self::type_info() || *ty == PgTypeInfo::with_name("text")
                        || *ty == PgTypeInfo::with_name("varchar")
                }
            }

            impl<'r> sqlx::Decode<'r, sqlx::Postgres> for $ty {
                fn decode(value: PgValueRef<'r>) -> Result<Self, BoxDynError> {
                    let s = <&str as sqlx::Decode<sqlx::Postgres>>::decode(value)?;
                    match s {
                        $($s => Ok($variant),)+
                        other => Err(format!(
                            "unknown {} value: {:?}",
                            $pg_name, other
                        ).into()),
                    }
                }
            }

            impl sqlx::Encode<'_, sqlx::Postgres> for $ty {
                fn encode_by_ref(
                    &self,
                    buf: &mut PgArgumentBuffer,
                ) -> Result<IsNull, BoxDynError> {
                    let s: &str = match self {
                        $($variant => $s,)+
                    };
                    <&str as sqlx::Encode<sqlx::Postgres>>::encode_by_ref(&s, buf)
                }
            }
        };
    }

    pg_enum!(OrgRole, "org_role",
        OrgRole::Admin   => "admin",
        OrgRole::Manager => "manager",
        OrgRole::Member  => "member",
    );

    pg_enum!(EntryState, "entry_state",
        EntryState::Open      => "open",
        EntryState::Submitted => "submitted",
        EntryState::Approved  => "approved",
        EntryState::Invoiced  => "invoiced",
    );

    pg_enum!(RoundDir, "round_dir",
        RoundDir::Up      => "up",
        RoundDir::Down    => "down",
        RoundDir::Nearest => "nearest",
    );

    pg_enum!(ProjectType, "project_type",
        ProjectType::TimeAndMaterials => "time_and_materials",
        ProjectType::FixedFee         => "fixed_fee",
        ProjectType::NonBillable      => "non_billable",
        ProjectType::Retainer         => "retainer",
    );

    pg_enum!(BudgetKind, "budget_kind",
        BudgetKind::Hours => "hours",
        BudgetKind::Money => "money",
        BudgetKind::None  => "none",
    );

    pg_enum!(ProjectRole, "project_role",
        ProjectRole::Lead       => "lead",
        ProjectRole::Freelancer => "freelancer",
        ProjectRole::Admin      => "admin",
    );
}
