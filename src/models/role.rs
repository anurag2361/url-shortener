use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum Role {
    // URL Management
    UrlCreator, // Can create new short URLs
    UrlViewer,  // Can view URL list
    UrlManager, // Can edit/delete URLs

    // QR Code Management
    QrCreator, // Can generate QR codes
    QrViewer,  // Can view QR codes
    QrManager, // Can regenerate/modify QR codes

    // Analytics
    AnalyticsViewer,  // Can view basic analytics
    AnalyticsManager, // Can view detailed analytics

    // User Management
    UserViewer,  // Can view users
    UserManager, // Can create/edit users

    // System
    SystemAdmin, // System configuration, logs, etc.

    // Special
    SuperUser, // Has all permissions
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Role::UrlCreator => write!(f, "URL Creator"),
            Role::UrlViewer => write!(f, "URL Viewer"),
            Role::UrlManager => write!(f, "URL Manager"),
            Role::QrCreator => write!(f, "QR Creator"),
            Role::QrViewer => write!(f, "QR Viewer"),
            Role::QrManager => write!(f, "QR Manager"),
            Role::AnalyticsViewer => write!(f, "Analytics Viewer"),
            Role::AnalyticsManager => write!(f, "Analytics Manager"),
            Role::UserViewer => write!(f, "User Viewer"),
            Role::UserManager => write!(f, "User Manager"),
            Role::SystemAdmin => write!(f, "System Administrator"),
            Role::SuperUser => write!(f, "Super User"),
        }
    }
}
