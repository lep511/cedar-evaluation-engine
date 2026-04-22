// Internal Marketing Team can access all user data fields
permit(
    principal == MarketingTeam::"InternalTeam",
    action == Action::"access",
    resource
);

// External Marketing Partner can access plansViewed (non-PII behavioral data)
permit(
    principal == MarketingTeam::"ExternalPartner",
    action == Action::"access",
    resource == UserDataField::"plansViewed"
);

// External Marketing Partner can access registrationDate (non-PII temporal data)
permit(
    principal == MarketingTeam::"ExternalPartner",
    action == Action::"access",
    resource == UserDataField::"registrationDate"
);
