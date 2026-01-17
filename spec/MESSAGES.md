# AtomicSettle Message Format Specification

**Version:** 0.1.0
**Status:** Draft
**Last Updated:** 2026-01-17

## Overview

This document defines the message formats used in the AtomicSettle protocol. Messages are serialized using Protocol Buffers (protobuf) for efficiency and schema evolution support.

## Table of Contents

1. [Common Types](#1-common-types)
2. [Settlement Messages](#2-settlement-messages)
3. [Lock Messages](#3-lock-messages)
4. [Notification Messages](#4-notification-messages)
5. [Control Messages](#5-control-messages)
6. [Error Handling](#6-error-handling)

---

## 1. Common Types

### 1.1 Identifiers

```protobuf
// Unique settlement identifier
message SettlementId {
    string value = 1;  // UUID v7 format (time-ordered)
}

// Participant identifier
message ParticipantId {
    string value = 1;  // ISO 9362 BIC or assigned identifier
}

// Account identifier
message AccountId {
    string participant_id = 1;  // Owner participant
    string account_number = 2;  // Account within participant
    string currency = 3;        // ISO 4217 currency code
}

// Lock identifier
message LockId {
    string value = 1;  // UUID v4
}
```

### 1.2 Monetary Types

```protobuf
// Monetary amount with currency
message Money {
    string value = 1;     // Decimal string, e.g., "1000000.00"
    string currency = 2;  // ISO 4217 currency code, e.g., "USD"
}

// FX rate between two currencies
message FxRate {
    string base_currency = 1;    // e.g., "USD"
    string quote_currency = 2;   // e.g., "EUR"
    string bid = 3;              // Buy price
    string ask = 4;              // Sell price
    string mid = 5;              // Mid-market rate
    google.protobuf.Timestamp valid_until = 6;
    string source = 7;           // Rate provider
}
```

### 1.3 Timestamps and Signatures

```protobuf
// Message envelope with common fields
message MessageEnvelope {
    string version = 1;          // Protocol version, e.g., "1.0"
    string message_type = 2;     // Type discriminator
    string message_id = 3;       // Unique message ID (UUID)
    string correlation_id = 4;   // Links related messages
    google.protobuf.Timestamp timestamp = 5;
    bytes signature = 6;         // Ed25519 signature over message content
    string signer_id = 7;        // Participant or coordinator ID
}

// Cryptographic signature
message Signature {
    string algorithm = 1;  // "Ed25519" or "ECDSA-P256"
    bytes value = 2;       // Signature bytes
    string key_id = 3;     // Key identifier for rotation
}
```

---

## 2. Settlement Messages

### 2.1 SettleRequest

Initiates a new settlement. Sent by participant to coordinator.

```protobuf
message SettleRequest {
    MessageEnvelope envelope = 1;

    // Required fields
    SettlementId settlement_id = 2;     // Client-generated
    string idempotency_key = 3;         // For duplicate detection
    ParticipantId sender = 4;
    ParticipantId receiver = 5;
    Money amount = 6;

    // Optional FX instruction
    FxInstruction fx_instruction = 7;

    // Compliance information
    ComplianceData compliance = 8;

    // Settlement legs (for multi-leg settlements)
    repeated SettlementLeg legs = 9;

    // Metadata
    map<string, string> metadata = 10;
}

message FxInstruction {
    enum FxMode {
        AT_SOURCE = 0;       // Sender converts
        AT_DESTINATION = 1;  // Receiver converts
        AT_COORDINATOR = 2;  // Coordinator converts
    }

    FxMode mode = 1;
    string target_currency = 2;   // If mode is AT_COORDINATOR
    FxRate locked_rate = 3;       // Pre-locked rate (optional)
    string rate_reference = 4;    // Reference for rate lookup
}

message ComplianceData {
    string purpose_code = 1;        // ISO 20022 purpose code
    string remittance_info = 2;     // Payment reference
    Debtor debtor = 3;              // Ultimate debtor info
    Creditor creditor = 4;          // Ultimate creditor info
    string regulatory_reporting = 5; // Jurisdiction-specific
}

message Debtor {
    string name = 1;
    string identifier = 2;          // Tax ID, LEI, etc.
    string identifier_type = 3;     // "LEI", "TAX_ID", etc.
    Address address = 4;
}

message Creditor {
    string name = 1;
    string identifier = 2;
    string identifier_type = 3;
    Address address = 4;
}

message Address {
    string street = 1;
    string city = 2;
    string postal_code = 3;
    string country = 4;  // ISO 3166-1 alpha-2
}

message SettlementLeg {
    int32 leg_number = 1;
    ParticipantId from_participant = 2;
    AccountId from_account = 3;
    ParticipantId to_participant = 4;
    AccountId to_account = 5;
    Money amount = 6;
    FxInstruction fx_instruction = 7;
}
```

### 2.2 SettleResponse

Response to settlement request. Sent by coordinator to participant.

```protobuf
message SettleResponse {
    MessageEnvelope envelope = 1;

    SettlementId settlement_id = 2;
    SettlementStatus status = 3;

    // Set if status is REJECTED
    RejectionReason rejection = 4;

    // Set if status is VALIDATED or later
    ValidatedSettlement validated = 5;

    // Coordinator signature over settlement details
    Signature coordinator_signature = 6;
}

enum SettlementStatus {
    INITIATED = 0;
    VALIDATED = 1;
    LOCKED = 2;
    COMMITTED = 3;
    SETTLED = 4;
    REJECTED = 5;
    FAILED = 6;
}

message RejectionReason {
    string code = 1;        // Error code
    string message = 2;     // Human-readable message
    string field = 3;       // Field that caused rejection (if applicable)
}

message ValidatedSettlement {
    SettlementId settlement_id = 1;
    google.protobuf.Timestamp validated_at = 2;

    // Final amounts after FX (if applicable)
    repeated SettlementLeg final_legs = 3;

    // Locked FX rate (if FX settlement)
    FxRate locked_fx_rate = 4;

    // Expiration for this validation
    google.protobuf.Timestamp expires_at = 5;
}
```

---

## 3. Lock Messages

### 3.1 LockRequest

Request to lock funds at a participant. Sent by coordinator to participant.

```protobuf
message LockRequest {
    MessageEnvelope envelope = 1;

    LockId lock_id = 2;
    SettlementId settlement_id = 3;

    // Account and amount to lock
    AccountId account = 4;
    Money amount = 5;

    // Lock parameters
    google.protobuf.Timestamp expires_at = 6;
    LockPriority priority = 7;

    // Coordinator signature
    Signature coordinator_signature = 8;
}

enum LockPriority {
    NORMAL = 0;
    HIGH = 1;      // Time-critical settlement
    SYSTEM = 2;    // System-level priority (netting, etc.)
}
```

### 3.2 LockResponse

Response to lock request. Sent by participant to coordinator.

```protobuf
message LockResponse {
    MessageEnvelope envelope = 1;

    LockId lock_id = 2;
    SettlementId settlement_id = 3;
    LockStatus status = 4;

    // If successful
    google.protobuf.Timestamp locked_at = 5;
    google.protobuf.Timestamp actual_expires_at = 6;

    // If failed
    LockFailureReason failure = 7;

    // Participant signature
    Signature participant_signature = 8;
}

enum LockStatus {
    ACQUIRED = 0;
    FAILED = 1;
    EXTENDED = 2;
    RELEASED = 3;
}

message LockFailureReason {
    enum Code {
        INSUFFICIENT_FUNDS = 0;
        ACCOUNT_BLOCKED = 1;
        LOCK_CONFLICT = 2;
        SYSTEM_ERROR = 3;
        LIMIT_EXCEEDED = 4;
    }

    Code code = 1;
    string message = 2;
    Money available_balance = 3;  // If INSUFFICIENT_FUNDS
}
```

### 3.3 LockRelease

Release a lock (on success or failure). Sent by coordinator to participant.

```protobuf
message LockRelease {
    MessageEnvelope envelope = 1;

    LockId lock_id = 2;
    SettlementId settlement_id = 3;

    ReleaseReason reason = 4;

    Signature coordinator_signature = 5;
}

enum ReleaseReason {
    SETTLEMENT_COMPLETE = 0;
    SETTLEMENT_FAILED = 1;
    LOCK_EXPIRED = 2;
    COORDINATOR_ABORT = 3;
}
```

---

## 4. Notification Messages

### 4.1 SettlementNotification

Notifies participants of settlement state changes.

```protobuf
message SettlementNotification {
    MessageEnvelope envelope = 1;

    SettlementId settlement_id = 2;
    SettlementStatus new_status = 3;
    SettlementStatus previous_status = 4;

    google.protobuf.Timestamp status_changed_at = 5;

    // Full settlement details (for COMMITTED/SETTLED)
    Settlement settlement = 6;

    // Failure info (for FAILED status)
    SettlementFailure failure = 7;

    Signature coordinator_signature = 8;
}

message Settlement {
    SettlementId id = 1;
    google.protobuf.Timestamp initiated_at = 2;
    google.protobuf.Timestamp settled_at = 3;

    repeated SettlementLeg legs = 4;

    // FX details if applicable
    FxDetails fx_details = 5;

    // Timing metrics
    TimingMetrics timing = 6;

    // Compliance acknowledgments
    repeated ComplianceAck compliance_acks = 7;
}

message FxDetails {
    FxRate rate_used = 1;
    Money source_amount = 2;
    Money converted_amount = 3;
    string conversion_reference = 4;
}

message TimingMetrics {
    int64 total_duration_ms = 1;
    int64 validation_duration_ms = 2;
    int64 lock_duration_ms = 3;
    int64 commit_duration_ms = 4;
}

message SettlementFailure {
    enum FailureCode {
        LOCK_TIMEOUT = 0;
        PARTICIPANT_UNAVAILABLE = 1;
        COORDINATOR_ERROR = 2;
        COMPLIANCE_REJECTED = 3;
        FX_RATE_EXPIRED = 4;
        NETTING_FAILURE = 5;
    }

    FailureCode code = 1;
    string message = 2;
    int32 failed_leg = 3;  // Which leg failed (if applicable)
    google.protobuf.Timestamp failed_at = 4;
}

message ComplianceAck {
    string check_type = 1;     // "SANCTIONS", "AML", etc.
    string result = 2;         // "PASS", "REVIEW", etc.
    string reference = 3;      // External reference
}
```

### 4.2 SettlementAcknowledgment

Participant acknowledges receipt of settlement notification.

```protobuf
message SettlementAcknowledgment {
    MessageEnvelope envelope = 1;

    SettlementId settlement_id = 2;
    SettlementStatus acknowledged_status = 3;

    // Participant's local reference
    string local_reference = 4;

    Signature participant_signature = 5;
}
```

---

## 5. Control Messages

### 5.1 Connect

Initial connection handshake.

```protobuf
message ConnectRequest {
    MessageEnvelope envelope = 1;

    ParticipantId participant_id = 2;
    repeated string supported_versions = 3;  // ["1.0", "1.1"]

    // Client certificate info
    string certificate_fingerprint = 4;

    // Capabilities
    repeated string capabilities = 5;  // ["FX", "NETTING", "MULTI_LEG"]
}

message ConnectResponse {
    MessageEnvelope envelope = 1;

    string selected_version = 2;
    bool success = 3;

    // If failed
    string failure_reason = 4;

    // Coordinator info
    CoordinatorInfo coordinator = 5;

    // Pending settlements to sync
    repeated SettlementId pending_settlements = 6;
}

message CoordinatorInfo {
    string coordinator_id = 1;
    string network_name = 2;  // "PRODUCTION", "SANDBOX", etc.
    repeated string supported_currencies = 3;
    google.protobuf.Timestamp server_time = 4;
}
```

### 5.2 Heartbeat

Keep-alive and sync mechanism.

```protobuf
message Heartbeat {
    MessageEnvelope envelope = 1;

    ParticipantId sender = 2;
    google.protobuf.Timestamp client_time = 3;

    // Current sequence numbers for sync
    uint64 last_settlement_seq = 4;
    uint64 last_notification_seq = 5;
}

message HeartbeatAck {
    MessageEnvelope envelope = 1;

    google.protobuf.Timestamp server_time = 2;

    // Missing messages to resend
    repeated uint64 missing_sequences = 3;
}
```

### 5.3 BalanceQuery

Query current position/balance.

```protobuf
message BalanceQuery {
    MessageEnvelope envelope = 1;

    ParticipantId participant_id = 2;
    repeated string currencies = 3;  // Empty = all currencies
}

message BalanceResponse {
    MessageEnvelope envelope = 1;

    repeated Balance balances = 2;
    google.protobuf.Timestamp as_of = 3;
}

message Balance {
    string currency = 1;
    string available = 2;     // Available for settlement
    string locked = 3;        // Currently locked
    string pending_in = 4;    // Incoming (not yet final)
    string pending_out = 5;   // Outgoing (not yet final)
    string total = 6;         // Total position
}
```

---

## 6. Error Handling

### 6.1 Error Response

Generic error response for any message.

```protobuf
message ErrorResponse {
    MessageEnvelope envelope = 1;

    string error_code = 2;
    string error_message = 3;

    // Original message that caused error
    string original_message_id = 4;
    string original_message_type = 5;

    // Retryable?
    bool retryable = 6;
    google.protobuf.Duration retry_after = 7;

    // Additional context
    map<string, string> details = 8;
}
```

### 6.2 Error Codes

| Code | Description | Retryable |
|------|-------------|-----------|
| `INVALID_MESSAGE` | Message failed validation | No |
| `INVALID_SIGNATURE` | Signature verification failed | No |
| `UNKNOWN_PARTICIPANT` | Participant not registered | No |
| `PARTICIPANT_OFFLINE` | Target participant unavailable | Yes |
| `RATE_LIMITED` | Too many requests | Yes |
| `COORDINATOR_BUSY` | Coordinator overloaded | Yes |
| `SETTLEMENT_NOT_FOUND` | Settlement ID unknown | No |
| `LOCK_NOT_FOUND` | Lock ID unknown | No |
| `DUPLICATE_REQUEST` | Idempotency key already used | No* |
| `VERSION_MISMATCH` | Protocol version incompatible | No |
| `INTERNAL_ERROR` | Unexpected coordinator error | Yes |

*DUPLICATE_REQUEST returns the original response, not an error.

---

## Appendix A: Protobuf Schema Files

Full protobuf schema files are available in the repository:

- `proto/common.proto` - Common types
- `proto/settlement.proto` - Settlement messages
- `proto/lock.proto` - Lock messages
- `proto/notification.proto` - Notification messages
- `proto/control.proto` - Control messages
- `proto/error.proto` - Error handling

## Appendix B: JSON Encoding

For debugging and logging, messages can be encoded as JSON using standard protobuf-to-JSON mapping:

```json
{
  "envelope": {
    "version": "1.0",
    "messageType": "SETTLE_REQUEST",
    "messageId": "550e8400-e29b-41d4-a716-446655440000",
    "timestamp": "2026-01-17T10:30:00Z"
  },
  "settlementId": {
    "value": "019456ab-1234-7def-8901-234567890abc"
  },
  "sender": {
    "value": "JPMORGAN_NY"
  },
  "receiver": {
    "value": "HSBC_LONDON"
  },
  "amount": {
    "value": "1000000.00",
    "currency": "USD"
  }
}
```

## Appendix C: Change Log

| Version | Date | Changes |
|---------|------|---------|
| 0.1.0 | 2026-01-17 | Initial draft |
