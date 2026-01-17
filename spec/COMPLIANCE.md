# AtomicSettle Compliance Specification

**Version:** 0.1.0
**Status:** Draft
**Last Updated:** 2026-01-17

## Overview

This document specifies the regulatory compliance requirements, AML/CFT controls, and jurisdictional considerations for the AtomicSettle protocol.

## Table of Contents

1. [Regulatory Framework](#1-regulatory-framework)
2. [AML/CFT Requirements](#2-amlcft-requirements)
3. [Sanctions Screening](#3-sanctions-screening)
4. [Transaction Monitoring](#4-transaction-monitoring)
5. [Reporting Obligations](#5-reporting-obligations)
6. [Data Protection](#6-data-protection)
7. [Jurisdictional Requirements](#7-jurisdictional-requirements)
8. [Participant Compliance](#8-participant-compliance)
9. [Coordinator Obligations](#9-coordinator-obligations)
10. [Audit and Examination](#10-audit-and-examination)

---

## 1. Regulatory Framework

### 1.1 Applicable Standards

AtomicSettle is designed to comply with international standards and major jurisdictional requirements:

| Standard | Description | Applicability |
|----------|-------------|---------------|
| **CPMI-IOSCO PFMI** | Principles for Financial Market Infrastructures | Coordinator operations |
| **FATF Recommendations** | AML/CFT standards | All participants |
| **ISO 20022** | Financial messaging standard | Message formats |
| **Basel III** | Capital and liquidity requirements | Bank participants |
| **PSD2/PSD3** | EU Payment Services Directive | EU participants |
| **Dodd-Frank Title VIII** | US FMI supervision | US systemically important |

### 1.2 Regulatory Classification

```
┌─────────────────────────────────────────────────────────────────┐
│                    REGULATORY CLASSIFICATION                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Coordinator Network:                                           │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │ • Payment System Operator (PSO)                          │   │
│  │ • Financial Market Infrastructure (FMI)                  │   │
│  │ • Systemically Important Payment System (SIPS)           │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
│  Participants:                                                  │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │ • Licensed banks/credit institutions                     │   │
│  │ • Payment institutions (EMI/PI)                          │   │
│  │ • Central banks                                          │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 1.3 Licensing Requirements

**Coordinator Operators MUST:**
- Hold appropriate payment system operator license
- Be supervised by relevant financial authority
- Meet capital and liquidity requirements
- Maintain business continuity arrangements

**Participants MUST:**
- Be licensed financial institutions
- Be in good regulatory standing
- Complete coordinator onboarding process
- Sign participation agreement

---

## 2. AML/CFT Requirements

### 2.1 Customer Due Diligence (CDD)

Each participant is responsible for CDD on their own customers. The coordinator relies on participant attestations:

```python
class ParticipantAttestation:
    """
    Attestation that participant has performed required CDD.
    """
    participant_id: str
    attestation_date: datetime

    # CDD program confirmation
    has_aml_program: bool
    has_kyc_procedures: bool
    has_sanctions_screening: bool
    has_transaction_monitoring: bool

    # Regulatory status
    primary_regulator: str
    license_number: str
    last_exam_date: datetime

    # Signature
    authorized_signatory: str
    signature: bytes
```

### 2.2 Enhanced Due Diligence (EDD)

EDD is required for:

| Trigger | Required Actions |
|---------|------------------|
| High-value settlements (>$1M equivalent) | Additional verification, senior approval |
| High-risk jurisdictions | Enhanced monitoring, source of funds |
| PEP involvement | Senior management approval, ongoing monitoring |
| Complex settlement structures | Purpose verification, beneficial ownership |
| Unusual patterns | Investigation, possible SAR filing |

### 2.3 Settlement Request Compliance Fields

```protobuf
message ComplianceInfo {
    // Purpose of settlement
    string purpose_code = 1;  // ISO 20022 purpose code
    string remittance_info = 2;

    // Originator information (FATF Travel Rule)
    Originator originator = 3;

    // Beneficiary information
    Beneficiary beneficiary = 4;

    // Compliance attestations
    bool sanctions_screened = 5;
    bool aml_verified = 6;

    // For EDD cases
    optional string edd_reference = 7;
    optional string approval_reference = 8;
}

message Originator {
    string name = 1;
    string account_number = 2;
    string address = 3;
    optional string national_id = 4;
    optional string date_of_birth = 5;
    optional string place_of_birth = 6;
}

message Beneficiary {
    string name = 1;
    string account_number = 2;
    optional string address = 3;
}
```

---

## 3. Sanctions Screening

### 3.1 Screening Requirements

All settlements MUST be screened against sanctions lists before execution:

```
Settlement Request
        │
        ▼
┌───────────────────┐
│ Sender Screening  │──► MATCH ──► BLOCK + REPORT
└─────────┬─────────┘
          │ CLEAR
          ▼
┌───────────────────┐
│Receiver Screening │──► MATCH ──► BLOCK + REPORT
└─────────┬─────────┘
          │ CLEAR
          ▼
┌───────────────────┐
│ Narrative Screen  │──► MATCH ──► REVIEW + HOLD
└─────────┬─────────┘
          │ CLEAR
          ▼
    Continue Settlement
```

### 3.2 Sanctions Lists

| List | Source | Update Frequency |
|------|--------|------------------|
| OFAC SDN | US Treasury | Real-time |
| OFAC Consolidated | US Treasury | Daily |
| EU Consolidated | European Commission | Daily |
| UN Consolidated | United Nations | Daily |
| HMT | UK Treasury | Daily |
| Local Lists | Participant jurisdiction | As required |

### 3.3 Screening Implementation

```python
class SanctionsScreener:
    def __init__(self, list_providers: List[SanctionsListProvider]):
        self.providers = list_providers
        self.cache = SanctionsCache()

    async def screen_settlement(
        self,
        settlement: SettleRequest
    ) -> ScreeningResult:
        results = []

        # Screen all parties
        for party in [settlement.originator, settlement.beneficiary]:
            party_result = await self.screen_party(party)
            results.append(party_result)

        # Screen remittance information
        narrative_result = await self.screen_narrative(
            settlement.compliance.remittance_info
        )
        results.append(narrative_result)

        return self.aggregate_results(results)

    async def screen_party(self, party: Party) -> PartyScreenResult:
        matches = []

        for provider in self.providers:
            provider_matches = await provider.search(
                name=party.name,
                address=party.address,
                identifiers=party.identifiers
            )
            matches.extend(provider_matches)

        return PartyScreenResult(
            party=party,
            matches=matches,
            status=self.determine_status(matches)
        )

    def determine_status(self, matches: List[Match]) -> ScreenStatus:
        if any(m.confidence > 0.95 for m in matches):
            return ScreenStatus.BLOCKED
        elif any(m.confidence > 0.70 for m in matches):
            return ScreenStatus.REVIEW_REQUIRED
        else:
            return ScreenStatus.CLEAR
```

### 3.4 False Positive Management

```python
class FalsePositiveRegistry:
    """
    Manage confirmed false positives to reduce screening friction.
    """

    async def register_false_positive(
        self,
        match_id: str,
        party_id: str,
        reviewer: str,
        justification: str,
        expiry: datetime
    ) -> FalsePositive:
        # Require dual approval for false positive registration
        fp = FalsePositive(
            match_id=match_id,
            party_id=party_id,
            registered_by=reviewer,
            justification=justification,
            expires_at=expiry,
            status=FalsePositiveStatus.PENDING_APPROVAL
        )

        await self.store(fp)
        await self.request_approval(fp)

        return fp

    async def is_known_false_positive(
        self,
        match: Match,
        party_id: str
    ) -> bool:
        fp = await self.find(match.list_entry_id, party_id)
        return (
            fp is not None
            and fp.status == FalsePositiveStatus.APPROVED
            and fp.expires_at > datetime.utcnow()
        )
```

---

## 4. Transaction Monitoring

### 4.1 Monitoring Requirements

The coordinator implements network-level transaction monitoring:

```python
class TransactionMonitor:
    def __init__(self):
        self.rules = RuleEngine()
        self.ml_model = AnomalyDetector()
        self.alert_queue = AlertQueue()

    async def monitor_settlement(
        self,
        settlement: Settlement
    ) -> MonitoringResult:
        alerts = []

        # Rule-based detection
        rule_alerts = await self.rules.evaluate(settlement)
        alerts.extend(rule_alerts)

        # ML-based anomaly detection
        anomaly_score = await self.ml_model.score(settlement)
        if anomaly_score > ANOMALY_THRESHOLD:
            alerts.append(AnomalyAlert(settlement, anomaly_score))

        # Queue alerts for investigation
        for alert in alerts:
            await self.alert_queue.enqueue(alert)

        return MonitoringResult(
            settlement_id=settlement.id,
            alerts=alerts,
            risk_score=self.calculate_risk_score(alerts)
        )
```

### 4.2 Detection Rules

| Rule Category | Examples |
|--------------|----------|
| **Velocity** | >10 settlements per hour, unusual time patterns |
| **Amount** | Large round amounts, structuring patterns |
| **Geographic** | High-risk corridors, unusual routing |
| **Behavioral** | Deviation from baseline, new counterparties |
| **Network** | Circular flows, hub patterns |

### 4.3 Alert Prioritization

```
┌─────────────────────────────────────────────────────────────┐
│                    ALERT PRIORITY MATRIX                     │
├─────────────────┬───────────────────────────────────────────┤
│                 │           RULE CONFIDENCE                  │
│  RISK SCORE     │   LOW      │   MEDIUM    │    HIGH        │
├─────────────────┼────────────┼─────────────┼────────────────┤
│  HIGH (>80)     │  MEDIUM    │    HIGH     │   CRITICAL     │
├─────────────────┼────────────┼─────────────┼────────────────┤
│  MEDIUM (50-80) │   LOW      │   MEDIUM    │    HIGH        │
├─────────────────┼────────────┼─────────────┼────────────────┤
│  LOW (<50)      │   INFO     │    LOW      │   MEDIUM       │
└─────────────────┴────────────┴─────────────┴────────────────┘
```

---

## 5. Reporting Obligations

### 5.1 Suspicious Activity Reporting

Participants are responsible for SAR/STR filing in their jurisdictions:

```python
class SuspiciousActivityReport:
    """
    SAR data structure for regulatory filing.
    """
    # Filing information
    filing_institution: str
    filing_date: datetime
    report_type: ReportType  # INITIAL, CONTINUING, JOINT

    # Subject information
    subjects: List[Subject]

    # Activity details
    activity_date_range: DateRange
    settlement_ids: List[str]
    total_amount: Money
    activity_description: str

    # Suspicious indicators
    indicators: List[SuspiciousIndicator]

    # Supporting documentation
    attachments: List[Attachment]
```

### 5.2 Regulatory Reporting

| Report Type | Frequency | Recipients |
|-------------|-----------|------------|
| Settlement Statistics | Daily | Primary regulator |
| Large Value Report | Real-time | FIU/FinCEN |
| Cross-Border Summary | Monthly | Central bank |
| Incident Report | As needed | All regulators |
| Annual Compliance Report | Annually | Primary regulator |

### 5.3 Travel Rule Compliance

FATF Travel Rule (Recommendation 16) requires transmission of originator/beneficiary information:

```python
class TravelRuleHandler:
    """
    Ensure Travel Rule compliance for all settlements.
    """

    THRESHOLD_USD = 3000  # FATF threshold

    def validate_travel_rule_data(
        self,
        settlement: SettleRequest
    ) -> ValidationResult:
        errors = []

        # Check if Travel Rule applies
        if settlement.amount_usd < self.THRESHOLD_USD:
            return ValidationResult(valid=True)

        # Validate originator information
        originator = settlement.compliance.originator
        if not originator.name:
            errors.append("Originator name required")
        if not originator.account_number:
            errors.append("Originator account required")
        if not originator.address:
            errors.append("Originator address required")

        # Validate beneficiary information
        beneficiary = settlement.compliance.beneficiary
        if not beneficiary.name:
            errors.append("Beneficiary name required")
        if not beneficiary.account_number:
            errors.append("Beneficiary account required")

        return ValidationResult(
            valid=len(errors) == 0,
            errors=errors
        )
```

---

## 6. Data Protection

### 6.1 Privacy Requirements

| Regulation | Jurisdiction | Key Requirements |
|------------|--------------|------------------|
| GDPR | EU/EEA | Consent, data minimization, right to erasure |
| CCPA/CPRA | California | Disclosure, opt-out, deletion rights |
| LGPD | Brazil | Consent, legitimate interest, data protection |
| PDPA | Singapore | Consent, purpose limitation, accuracy |

### 6.2 Data Classification

```
┌─────────────────────────────────────────────────────────────┐
│                    DATA CLASSIFICATION                       │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  RESTRICTED (Highest Protection)                            │
│  ├── Personal identifiers (SSN, passport)                   │
│  ├── Authentication credentials                              │
│  └── Private keys                                           │
│                                                             │
│  CONFIDENTIAL                                               │
│  ├── Settlement details                                     │
│  ├── Account numbers                                        │
│  ├── Beneficiary information                                │
│  └── Compliance data                                        │
│                                                             │
│  INTERNAL                                                   │
│  ├── Aggregate statistics                                   │
│  ├── System configuration                                   │
│  └── Audit logs (anonymized)                                │
│                                                             │
│  PUBLIC                                                     │
│  ├── Protocol specification                                 │
│  ├── API documentation                                      │
│  └── Participant list (if disclosed)                        │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 6.3 Data Retention

| Data Category | Retention Period | Justification |
|---------------|------------------|---------------|
| Settlement records | 10 years | Regulatory requirement |
| Audit logs | 7 years | Compliance/legal |
| Screening results | 5 years | AML record keeping |
| Personal data | As needed | GDPR minimization |
| System logs | 90 days | Operational |

### 6.4 Cross-Border Data Transfers

```python
class DataTransferValidator:
    """
    Validate cross-border data transfers comply with regulations.
    """

    # Adequacy decisions
    ADEQUATE_JURISDICTIONS = {
        "EU": ["EEA", "UK", "CH", "JP", "KR", "NZ", "CA", "IL", "AR", "UY"],
        "UK": ["EEA", "CH", "JP", "KR", "NZ", "CA", "IL", "AR", "UY", "US"],
    }

    def validate_transfer(
        self,
        source_jurisdiction: str,
        destination_jurisdiction: str,
        data_category: DataCategory
    ) -> TransferValidation:
        # Check adequacy
        if destination_jurisdiction in self.ADEQUATE_JURISDICTIONS.get(source_jurisdiction, []):
            return TransferValidation(
                allowed=True,
                mechanism="ADEQUACY_DECISION"
            )

        # Check for SCCs or other mechanisms
        if self.has_standard_contractual_clauses(source_jurisdiction, destination_jurisdiction):
            return TransferValidation(
                allowed=True,
                mechanism="STANDARD_CONTRACTUAL_CLAUSES"
            )

        # Check for binding corporate rules
        if self.has_bcr():
            return TransferValidation(
                allowed=True,
                mechanism="BINDING_CORPORATE_RULES"
            )

        return TransferValidation(
            allowed=False,
            mechanism=None,
            reason="No valid transfer mechanism"
        )
```

---

## 7. Jurisdictional Requirements

### 7.1 United States

**Applicable Regulations:**
- Bank Secrecy Act (BSA)
- USA PATRIOT Act
- Dodd-Frank Act (Title VIII for FMIs)
- FinCEN regulations

**Key Requirements:**
```python
US_REQUIREMENTS = {
    "ctr_threshold": 10000,  # Currency Transaction Report
    "sar_threshold": 5000,   # SAR threshold for banks
    "travel_rule_threshold": 3000,
    "cip_required": True,     # Customer Identification Program
    "ofac_screening": True,   # Mandatory OFAC screening
    "314a_participation": True,  # FinCEN information sharing
}
```

### 7.2 European Union

**Applicable Regulations:**
- 6th Anti-Money Laundering Directive (6AMLD)
- Payment Services Directive 2 (PSD2)
- Settlement Finality Directive
- GDPR

**Key Requirements:**
```python
EU_REQUIREMENTS = {
    "str_threshold": 0,  # No threshold for STRs
    "travel_rule_threshold": 1000,  # EUR
    "strong_customer_auth": True,  # SCA for payments
    "data_localization": False,  # With adequacy
    "beneficial_ownership": True,  # UBO registers
}
```

### 7.3 United Kingdom

**Applicable Regulations:**
- Money Laundering Regulations 2017
- Payment Services Regulations 2017
- FCA Handbook

### 7.4 Singapore

**Applicable Regulations:**
- Payment Services Act 2019
- MAS Notice PSN02 (AML/CFT)
- Personal Data Protection Act

### 7.5 Jurisdiction Matrix

| Requirement | US | EU | UK | SG | HK | JP |
|-------------|----|----|----|----|----|----|
| Travel Rule Threshold | $3K | €1K | £1K | S$1.5K | HK$8K | ¥100K |
| STR Threshold | $5K | None | None | None | None | None |
| Beneficial Ownership | Yes | Yes | Yes | Yes | Yes | Yes |
| Data Localization | No | Partial | No | Partial | Partial | Yes |
| Real-time Screening | Yes | Yes | Yes | Yes | Yes | Yes |

---

## 8. Participant Compliance

### 8.1 Onboarding Requirements

```
┌─────────────────────────────────────────────────────────────┐
│              PARTICIPANT ONBOARDING CHECKLIST                │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  1. REGULATORY VERIFICATION                                 │
│     □ License verification                                  │
│     □ Regulatory standing confirmation                      │
│     □ Sanctions check on institution                        │
│     □ Adverse media screening                               │
│                                                             │
│  2. COMPLIANCE ASSESSMENT                                   │
│     □ AML program review                                    │
│     □ Sanctions screening capabilities                      │
│     □ Transaction monitoring capabilities                   │
│     □ SAR/STR filing procedures                             │
│                                                             │
│  3. TECHNICAL VERIFICATION                                  │
│     □ Security assessment                                   │
│     □ Connectivity testing                                  │
│     □ Message format validation                             │
│     □ Failover testing                                      │
│                                                             │
│  4. LEGAL DOCUMENTATION                                     │
│     □ Participation agreement                               │
│     □ Compliance attestation                                │
│     □ Data processing agreement                             │
│     □ Security addendum                                     │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 8.2 Ongoing Compliance Obligations

Participants MUST:

1. **Maintain AML Program**: Active, risk-based AML/CFT program
2. **Screen Transactions**: All settlements screened before submission
3. **Monitor Activity**: Ongoing monitoring of settlement patterns
4. **Report Suspicious Activity**: File SARs/STRs as required
5. **Respond to Inquiries**: Cooperate with coordinator investigations
6. **Annual Attestation**: Renew compliance attestation annually

### 8.3 Compliance Attestation

```python
class AnnualAttestation:
    """
    Annual compliance attestation required from all participants.
    """
    participant_id: str
    attestation_period: DateRange

    # Program attestations
    aml_program_current: bool
    sanctions_lists_updated: bool
    staff_training_completed: bool
    independent_audit_completed: bool

    # Regulatory status
    no_enforcement_actions: bool
    no_license_restrictions: bool

    # Changes
    material_changes: List[str]

    # Certification
    certifying_officer: str
    certification_date: datetime
    signature: bytes
```

---

## 9. Coordinator Obligations

### 9.1 Regulatory Engagement

The coordinator MUST:

- Maintain open dialogue with supervisory authorities
- Provide requested information within required timeframes
- Notify regulators of material changes
- Participate in examinations and audits

### 9.2 Network Monitoring

```python
class NetworkComplianceMonitor:
    """
    Coordinator-level compliance monitoring.
    """

    async def daily_compliance_check(self) -> ComplianceReport:
        report = ComplianceReport(date=datetime.utcnow())

        # Check participant attestations
        report.attestation_status = await self.check_attestations()

        # Review screening effectiveness
        report.screening_metrics = await self.get_screening_metrics()

        # Monitor alert volumes
        report.alert_metrics = await self.get_alert_metrics()

        # Check for regulatory updates
        report.regulatory_updates = await self.check_regulatory_updates()

        return report

    async def check_attestations(self) -> AttestationStatus:
        participants = await self.get_all_participants()

        expired = []
        expiring_soon = []

        for p in participants:
            attestation = await self.get_attestation(p.id)
            if attestation.is_expired():
                expired.append(p.id)
            elif attestation.expires_within_days(30):
                expiring_soon.append(p.id)

        return AttestationStatus(
            expired=expired,
            expiring_soon=expiring_soon
        )
```

### 9.3 Incident Management

```python
class ComplianceIncidentHandler:
    """
    Handle compliance-related incidents.
    """

    async def handle_sanctions_match(
        self,
        settlement_id: str,
        match: SanctionsMatch
    ) -> IncidentResponse:
        # 1. Block settlement immediately
        await self.block_settlement(settlement_id)

        # 2. Create incident record
        incident = await self.create_incident(
            type=IncidentType.SANCTIONS_MATCH,
            settlement_id=settlement_id,
            details=match
        )

        # 3. Notify compliance team
        await self.notify_compliance_team(incident)

        # 4. Notify affected participants
        await self.notify_participants(incident)

        # 5. Preserve evidence
        await self.preserve_evidence(incident)

        # 6. Regulatory notification (if required)
        if match.requires_immediate_notification():
            await self.notify_regulators(incident)

        return IncidentResponse(
            incident_id=incident.id,
            status=IncidentStatus.UNDER_INVESTIGATION
        )
```

---

## 10. Audit and Examination

### 10.1 Internal Audit

Annual internal audit covering:

- AML/CFT program effectiveness
- Sanctions screening accuracy
- Transaction monitoring calibration
- Regulatory reporting completeness
- Data protection compliance

### 10.2 External Audit

Independent external audit by qualified auditor:

| Audit Type | Frequency | Scope |
|------------|-----------|-------|
| Financial | Annual | Financial statements |
| SOC 2 Type II | Annual | Security controls |
| AML Program | Annual | AML/CFT effectiveness |
| Technology | Biennial | System security |

### 10.3 Regulatory Examination

Cooperation with regulatory examinations:

```python
class ExaminationSupport:
    """
    Support regulatory examinations.
    """

    async def prepare_examination_package(
        self,
        examiner_requests: List[DataRequest]
    ) -> ExaminationPackage:
        package = ExaminationPackage()

        for request in examiner_requests:
            # Validate request authorization
            if not self.validate_authorization(request):
                continue

            # Gather requested data
            data = await self.gather_data(request)

            # Apply redactions if needed
            redacted_data = self.apply_required_redactions(data)

            package.add(request.id, redacted_data)

        # Log examination access
        await self.log_examination_access(package)

        return package
```

### 10.4 Audit Trail Requirements

All compliance-relevant actions MUST be logged:

```python
class ComplianceAuditLog:
    """
    Immutable audit trail for compliance actions.
    """

    REQUIRED_EVENTS = [
        "SCREENING_PERFORMED",
        "SCREENING_RESULT",
        "ALERT_GENERATED",
        "ALERT_DISPOSITION",
        "SAR_FILED",
        "ATTESTATION_SUBMITTED",
        "PARTICIPANT_SUSPENDED",
        "SETTLEMENT_BLOCKED",
        "REGULATORY_INQUIRY",
        "DATA_ACCESS",
    ]

    async def log_event(self, event: ComplianceEvent) -> None:
        entry = AuditEntry(
            event_id=uuid7(),
            timestamp=datetime.utcnow(),
            event_type=event.type,
            actor=event.actor,
            details=event.details,
            settlement_id=event.settlement_id,
        )

        # Sign entry
        entry.signature = self.sign(entry)

        # Append to immutable log
        await self.append(entry)
```

---

## Appendix A: Compliance Checklist

### Pre-Production Compliance Review

- [ ] AML/CFT program documentation complete
- [ ] Sanctions screening integrated and tested
- [ ] Transaction monitoring rules calibrated
- [ ] Regulatory reporting automated
- [ ] Travel Rule data capture implemented
- [ ] Data protection impact assessment completed
- [ ] Participant onboarding procedures documented
- [ ] Compliance training completed
- [ ] Independent compliance review performed
- [ ] Regulatory notifications submitted

---

## Appendix B: Regulatory Contacts

| Jurisdiction | Authority | Contact |
|--------------|-----------|---------|
| US | FinCEN | frc@fincen.gov |
| US | Federal Reserve | FMI supervision |
| EU | ECB | Payment systems oversight |
| UK | FCA | Authorizations |
| UK | Bank of England | FMI supervision |
| Singapore | MAS | Payment services |

---

## Appendix C: Change Log

| Version | Date | Changes |
|---------|------|---------|
| 0.1.0 | 2026-01-17 | Initial draft |
