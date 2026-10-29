//! Static in-form setup guides, as provider data on the catalogue listing (research R10,
//! contract `provider-credentials-api.md` §1). The form renders whatever guide a provider has;
//! there is no provider-id branch in the WebUI.

use serde::Serialize;

use crate::config::provider::ProviderId;

/// One numbered step of a guide.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct GuideStep {
    pub title: String,
    pub body: String,
}

/// A copyable policy or snippet.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct GuidePolicy {
    pub language: String,
    pub code: String,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct GuideLink {
    pub label: String,
    pub url: String,
}

/// What the connect form shows above the fields. Static metadata, never stored or editable.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct ProviderGuide {
    pub intro: String,
    pub steps: Vec<GuideStep>,
    pub policy: GuidePolicy,
    pub links: Vec<GuideLink>,
}

/// The exact minimal policy: `ListFoundationModels` is the Test action's read permission, and
/// `InvokeModel` runs chat, embeddings and images. Copied verbatim by the form and the docs.
pub const BEDROCK_POLICY: &str = "{\n  \"Version\": \"2012-10-17\",\n  \"Statement\": [{\n    \"Effect\": \"Allow\",\n    \"Action\": [\"bedrock:InvokeModel\", \"bedrock:ListFoundationModels\"],\n    \"Resource\": \"*\"\n  }]\n}";

/// The guide for a provider, if it has one. Only Amazon Bedrock has one today.
pub fn for_provider(id: &ProviderId) -> Option<ProviderGuide> {
    if *id == ProviderId::AMAZON_BEDROCK {
        Some(bedrock())
    } else if *id == ProviderId::AMAZON_BEDROCK_SSO {
        Some(bedrock_sso())
    } else {
        None
    }
}

fn step(title: &str, body: &str) -> GuideStep {
    GuideStep {
        title: title.into(),
        body: body.into(),
    }
}

fn link(label: &str, url: &str) -> GuideLink {
    GuideLink {
        label: label.into(),
        url: url.into(),
    }
}

fn bedrock_sso() -> ProviderGuide {
    ProviderGuide {
        intro: "Use an AWS CLI profile authenticated through IAM Identity Center (AWS SSO).".into(),
        steps: vec![
            step(
                "Configure an SSO profile",
                "Run `aws configure sso` and choose the AWS account, permission set and region that can use Bedrock.",
            ),
            step(
                "Sign in on this machine",
                "Run `aws sso login --profile <profile>` as the operating-system user that runs Vizier. Vizier never reads or stores the SSO cache.",
            ),
            step(
                "Enter the profile",
                "Enter that AWS CLI profile name and the Bedrock region below. Vizier obtains short-lived role credentials only when signing a request.",
            ),
            step(
                "Enable model access",
                "Bedrock → Model access: enable the chat, embedding and image models you will use. Anthropic models may need a use-case form.",
            ),
        ],
        policy: GuidePolicy {
            language: "json".into(),
            code: BEDROCK_POLICY.into(),
        },
        links: vec![
            link(
                "AWS CLI SSO",
                "https://docs.aws.amazon.com/cli/latest/userguide/sso-configure-profile-token.html",
            ),
            link(
                "Bedrock model access",
                "https://console.aws.amazon.com/bedrock/home#/modelaccess",
            ),
            link(
                "Vizier provider docs",
                "https://www.vizier.rs/docs/configuration/providers#amazon-bedrock-sso",
            ),
        ],
    }
}

fn bedrock() -> ProviderGuide {
    ProviderGuide {
        intro: "Use an IAM access key pair with permission to invoke Bedrock models.".into(),
        steps: vec![
            step(
                "Create an IAM access key",
                "IAM → Users → Security credentials → Create access key. Choose “Application \
                 running outside AWS”. The secret is shown once.",
            ),
            step(
                "Attach the minimal policy",
                "Allow the actions below. bedrock:ListFoundationModels is used by Test; \
                 bedrock:InvokeModel runs chat, embeddings and images.",
            ),
            step(
                "Enable model access",
                "Bedrock → Model access: enable the chat, embedding and image models you will \
                 use. Anthropic models may need a use-case form.",
            ),
            step(
                "Choose a region",
                "Use a Bedrock region where the model is offered, e.g. us-east-1. Cross-region \
                 ids (us., eu., apac., global.) are accepted as model ids.",
            ),
            step(
                "Paste the values",
                "Enter the access key ID, secret access key and region. Add the session token \
                 only for temporary credentials.",
            ),
        ],
        policy: GuidePolicy {
            language: "json".into(),
            code: BEDROCK_POLICY.into(),
        },
        links: vec![
            link(
                "AWS IAM users",
                "https://console.aws.amazon.com/iam/home#/users",
            ),
            link(
                "Bedrock model access",
                "https://console.aws.amazon.com/bedrock/home#/modelaccess",
            ),
            link(
                "AWS supported models",
                "https://docs.aws.amazon.com/bedrock/latest/userguide/models-supported.html",
            ),
            link(
                "Vizier provider docs",
                "https://www.vizier.rs/docs/configuration/providers#amazon-bedrock",
            ),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bedrock_sso_has_the_same_permission_guide() {
        let guide = for_provider(&ProviderId::AMAZON_BEDROCK_SSO).unwrap();
        assert!(guide.intro.contains("AWS CLI"));
        assert!(
            guide
                .steps
                .iter()
                .any(|step| step.body.contains("aws sso login"))
        );
    }

    #[test]
    fn only_bedrock_has_a_guide_and_it_carries_the_exact_policy() {
        assert!(for_provider(&ProviderId::AMAZON_BEDROCK).is_some());
        assert!(for_provider(&ProviderId::OPENAI).is_none());
        let guide = for_provider(&ProviderId::AMAZON_BEDROCK).unwrap();
        assert_eq!(guide.steps.len(), 5);
        assert_eq!(guide.policy.language, "json");
        assert_eq!(guide.policy.code, BEDROCK_POLICY);
        assert!(guide.policy.code.contains("bedrock:InvokeModel"));
        assert!(guide.policy.code.contains("bedrock:ListFoundationModels"));
        assert!(guide.policy.code.contains("\"Resource\": \"*\""));
        assert_eq!(guide.links.len(), 4);
        assert!(
            guide
                .links
                .iter()
                .any(|l| l.url.ends_with("#amazon-bedrock"))
        );
        let json = serde_json::to_value(&guide).unwrap();
        assert_eq!(json["policy"]["code"], BEDROCK_POLICY);
    }
}
