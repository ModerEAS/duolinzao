use uuid::Uuid;

use crate::error::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Perm {
    MealSelfRead,
    MealSelfWrite,
    MealAdminOverride,
    ShiftSelfWrite,
    MenuRead,
    MenuWrite,
    RatingSelfWrite,
    RatingAdminRead,
    RatingStrongAlertConfig,
    CommentAdminRead,
    OptimizerRead,
    UserInvite,
    UserRecover,
    DeviceApprove,
    AuditRead,
    ExportRun,
    SystemSuper,
}

impl Perm {
    pub fn code(self) -> &'static str {
        match self {
            Self::MealSelfRead => "meal.self_read",
            Self::MealSelfWrite => "meal.self_write",
            Self::MealAdminOverride => "meal.admin_override",
            Self::ShiftSelfWrite => "shift.self_write",
            Self::MenuRead => "menu.read",
            Self::MenuWrite => "menu.write",
            Self::RatingSelfWrite => "rating.self_write",
            Self::RatingAdminRead => "rating.admin_read",
            Self::RatingStrongAlertConfig => "rating.strong_alert_config",
            Self::CommentAdminRead => "comment.admin_read",
            Self::OptimizerRead => "optimizer.read",
            Self::UserInvite => "user.invite",
            Self::UserRecover => "user.recover",
            Self::DeviceApprove => "device.approve",
            Self::AuditRead => "audit.read",
            Self::ExportRun => "export.run",
            Self::SystemSuper => "system.super",
        }
    }

    pub fn needs_canteen(self) -> bool {
        matches!(
            self,
            Self::MealAdminOverride
                | Self::MenuRead
                | Self::MenuWrite
                | Self::RatingAdminRead
                | Self::CommentAdminRead
                | Self::OptimizerRead
        )
    }
}

const EMPLOYEE: &[Perm] = &[
    Perm::MealSelfRead,
    Perm::MealSelfWrite,
    Perm::ShiftSelfWrite,
    Perm::RatingSelfWrite,
    Perm::DeviceApprove,
];

const CANTEEN: &[Perm] = &[
    Perm::MenuRead,
    Perm::MenuWrite,
    Perm::RatingAdminRead,
    Perm::CommentAdminRead,
    Perm::OptimizerRead,
    Perm::DeviceApprove,
];

const ADMIN: &[Perm] = &[
    Perm::MealAdminOverride,
    Perm::RatingAdminRead,
    Perm::CommentAdminRead,
    Perm::UserInvite,
    Perm::DeviceApprove,
];

const SUPER: &[Perm] = &[
    Perm::MealSelfRead,
    Perm::MealSelfWrite,
    Perm::MealAdminOverride,
    Perm::ShiftSelfWrite,
    Perm::MenuRead,
    Perm::MenuWrite,
    Perm::RatingSelfWrite,
    Perm::RatingAdminRead,
    Perm::RatingStrongAlertConfig,
    Perm::CommentAdminRead,
    Perm::OptimizerRead,
    Perm::UserInvite,
    Perm::UserRecover,
    Perm::DeviceApprove,
    Perm::AuditRead,
    Perm::ExportRun,
    Perm::SystemSuper,
];

pub fn grants(role: &str) -> &'static [Perm] {
    match role {
        "employee" => EMPLOYEE,
        "canteen" => CANTEEN,
        "admin" => ADMIN,
        "super_admin" => SUPER,
        _ => &[],
    }
}

#[derive(Debug, Clone)]
pub struct Assignment {
    pub role: String,
    pub canteen_ids: Vec<Uuid>,
    pub source_label: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AuthHit {
    pub role: String,
    pub scope: String,
}

fn scope_of(assignment: &Assignment) -> String {
    if assignment.role == "super_admin" {
        return "*".to_string();
    }
    if assignment.canteen_ids.is_empty() {
        return "self".to_string();
    }
    assignment
        .canteen_ids
        .iter()
        .map(|id| id.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

pub fn authorize(
    assignments: &[Assignment],
    perm: Perm,
    canteen: Option<Uuid>,
) -> Result<AuthHit, AppError> {
    for assignment in assignments {
        if !grants(&assignment.role).contains(&perm) {
            continue;
        }
        if assignment.role == "super_admin" {
            return Ok(AuthHit {
                role: assignment.role.clone(),
                scope: "*".into(),
            });
        }
        if perm.needs_canteen() {
            let Some(canteen_id) = canteen else {
                continue;
            };
            if assignment.canteen_ids.contains(&canteen_id) {
                return Ok(AuthHit {
                    role: assignment.role.clone(),
                    scope: scope_of(assignment),
                });
            }
        } else {
            return Ok(AuthHit {
                role: assignment.role.clone(),
                scope: scope_of(assignment),
            });
        }
    }
    Err(AppError::forbidden())
}

pub fn authorize_any(
    assignments: &[Assignment],
    perms: &[Perm],
    canteen: Option<Uuid>,
) -> Result<AuthHit, AppError> {
    let mut last = AppError::forbidden();
    for perm in perms {
        match authorize(assignments, *perm, canteen) {
            Ok(hit) => return Ok(hit),
            Err(err) => last = err,
        }
    }
    Err(last)
}

pub fn permission_codes(assignments: &[Assignment]) -> Vec<&'static str> {
    let mut codes = Vec::new();
    for assignment in assignments {
        for perm in grants(&assignment.role) {
            let code = perm.code();
            if !codes.contains(&code) {
                codes.push(code);
            }
        }
    }
    codes
}

pub fn can_see_strong_alert(
    assignments: &[Assignment],
    canteen_id: Uuid,
) -> Result<AuthHit, AppError> {
    if let Some(assignment) = assignments.iter().find(|item| item.role == "super_admin") {
        return Ok(AuthHit {
            role: assignment.role.clone(),
            scope: "*".into(),
        });
    }
    if let Some(assignment) = assignments
        .iter()
        .find(|item| item.role == "canteen" && item.canteen_ids.contains(&canteen_id))
    {
        return Ok(AuthHit {
            role: assignment.role.clone(),
            scope: scope_of(assignment),
        });
    }
    Err(AppError::forbidden())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labeled(role: &str, canteens: Vec<Uuid>, label: &str) -> Assignment {
        Assignment {
            role: role.into(),
            canteen_ids: canteens,
            source_label: Some(label.into()),
        }
    }

    #[test]
    fn label_is_not_an_authorization_engine() {
        let canteen = Uuid::new_v4();
        let employee = labeled("employee", vec![], "后勤主管");
        assert!(authorize(&[employee], Perm::MenuWrite, Some(canteen)).is_err());
        assert!(authorize(
            &[labeled("employee", vec![], "后勤主管")],
            Perm::UserInvite,
            None
        )
        .is_err());
    }

    #[test]
    fn canteen_scope_does_not_cross_canteens() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let staff = labeled("canteen", vec![a], "");
        assert!(authorize(&[staff.clone()], Perm::MenuWrite, Some(a)).is_ok());
        assert!(authorize(&[staff], Perm::MenuWrite, Some(b)).is_err());
        let admin = labeled("admin", vec![a], "标签");
        assert!(authorize(&[admin.clone()], Perm::MealAdminOverride, Some(a)).is_ok());
        assert!(authorize(&[admin.clone()], Perm::UserInvite, None).is_ok());
        assert!(authorize(&[admin], Perm::MenuWrite, Some(a)).is_err());
    }

    #[test]
    fn super_admin_scope_is_global() {
        let hit = authorize(
            &[Assignment {
                role: "super_admin".into(),
                canteen_ids: vec![],
                source_label: None,
            }],
            Perm::ExportRun,
            None,
        )
        .unwrap();
        assert_eq!(hit.scope, "*");
        assert_eq!(hit.role, "super_admin");
    }
}
