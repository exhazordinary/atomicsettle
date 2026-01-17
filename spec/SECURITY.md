# AtomicSettle Security Specification

**Version:** 0.1.0
**Status:** Draft
**Last Updated:** 2026-01-17

## Overview

This document specifies the security requirements, cryptographic protocols, and threat mitigations for the AtomicSettle protocol.

## Table of Contents

1. [Security Principles](#1-security-principles)
2. [Authentication](#2-authentication)
3. [Cryptographic Primitives](#3-cryptographic-primitives)
4. [Transport Security](#4-transport-security)
5. [Message Security](#5-message-security)
6. [Key Management](#6-key-management)
7. [Access Control](#7-access-control)
8. [Audit and Logging](#8-audit-and-logging)
9. [Threat Model](#9-threat-model)
10. [Incident Response](#10-incident-response)

---

## 1. Security Principles

### 1.1 Defense in Depth

Multiple layers of security controls:

```
┌─────────────────────────────────────────────────────┐
│                 Network Security                     │
│  ┌───────────────────────────────────────────────┐  │
│  │              Transport Security                │  │
│  │  ┌─────────────────────────────────────────┐  │  │
│  │  │          Message Security               │  │  │
│  │  │  ┌───────────────────────────────────┐  │  │  │
│  │  │  │       Application Security        │  │  │  │
│  │  │  │  ┌─────────────────────────────┐  │  │  │  │
│  │  │  │  │      Data Security          │  │  │  │  │
│  │  │  │  └─────────────────────────────┘  │  │  │  │
│  │  │  └───────────────────────────────────┘  │  │  │
│  │  └─────────────────────────────────────────┘  │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

### 1.2 Core Security Principles

| Principle | Description |
|-----------|-------------|
| **Least Privilege** | Components have minimal necessary permissions |
| **Zero Trust** | All connections authenticated and encrypted |
| **Fail Secure** | Failures result in secure (denied) state |
| **Auditability** | All security-relevant events logged |
| **Key Rotation** | Regular cryptographic key rotation |
| **Separation of Duties** | No single point of compromise |

### 1.3 Security Objectives

1. **Confidentiality**: Settlement details visible only to involved parties
2. **Integrity**: Messages cannot be modified in transit or at rest
3. **Authenticity**: All messages verifiably from claimed sender
4. **Non-repudiation**: Participants cannot deny signed actions
5. **Availability**: System resilient to DoS attacks

---

## 2. Authentication

### 2.1 Participant Authentication

Participants authenticate using mutual TLS (mTLS) with X.509 certificates:

```
┌────────────────┐                    ┌────────────────┐
│  Participant   │                    │  Coordinator   │
└───────┬────────┘                    └───────┬────────┘
        │                                     │
        │ ClientHello + supported_versions    │
        │────────────────────────────────────►│
        │                                     │
        │ ServerHello + Certificate +         │
        │ CertificateRequest                  │
        │◄────────────────────────────────────│
        │                                     │
        │ Certificate + CertificateVerify +   │
        │ Finished                            │
        │────────────────────────────────────►│
        │                                     │
        │ Finished                            │
        │◄────────────────────────────────────│
        │                                     │
        │       (Encrypted channel)           │
        │◄───────────────────────────────────►│
```

### 2.2 Certificate Requirements

**Participant Certificates:**

| Field | Requirement |
|-------|-------------|
| Subject CN | Participant ID (e.g., "JPMORGAN_NY") |
| Subject O | Legal entity name |
| Key Algorithm | ECDSA P-256 or P-384 |
| Key Size | 256 bits minimum |
| Validity | Maximum 2 years |
| Extensions | Extended Key Usage: clientAuth |

**Coordinator Certificates:**

| Field | Requirement |
|-------|-------------|
| Subject CN | Coordinator domain |
| Subject O | Coordinator operator |
| Key Algorithm | ECDSA P-256 or P-384 |
| Key Size | 256 bits minimum |
| Validity | Maximum 1 year |
| Extensions | Extended Key Usage: serverAuth |
| SAN | All coordinator hostnames |

### 2.3 Certificate Authority

AtomicSettle operates a private PKI:

```
                    ┌─────────────────┐
                    │    Root CA      │
                    │  (Offline HSM)  │
                    └────────┬────────┘
                             │
              ┌──────────────┼──────────────┐
              │              │              │
    ┌─────────▼─────────┐ ┌──▼───────────┐ ┌▼────────────────┐
    │ Coordinator CA    │ │Participant CA│ │  Timestamp CA   │
    │                   │ │              │ │                 │
    └─────────┬─────────┘ └──────┬───────┘ └────────┬────────┘
              │                  │                   │
     Coordinator certs    Bank certs          TSA certs
```

### 2.4 Certificate Validation

On every connection:

```python
def validate_certificate(cert, expected_participant_id=None):
    # 1. Chain validation
    if not validate_chain(cert, TRUSTED_ROOTS):
        raise InvalidCertificateChain()

    # 2. Expiration check
    if cert.not_after < now() or cert.not_before > now():
        raise CertificateExpired()

    # 3. Revocation check (OCSP)
    if is_revoked(cert):
        raise CertificateRevoked()

    # 4. Participant ID match (if expected)
    if expected_participant_id:
        cert_participant = extract_participant_id(cert)
        if cert_participant != expected_participant_id:
            raise ParticipantMismatch()

    # 5. Usage constraints
    if 'clientAuth' not in cert.extended_key_usage:
        raise InvalidKeyUsage()

    return True
```

---

## 3. Cryptographic Primitives

### 3.1 Approved Algorithms

| Purpose | Algorithm | Parameters |
|---------|-----------|------------|
| Digital Signature | Ed25519 | Curve25519 |
| Digital Signature (alt) | ECDSA | P-256, P-384 |
| Key Agreement | X25519 | Curve25519 |
| Symmetric Encryption | AES-256-GCM | 256-bit key, 96-bit nonce |
| Hashing | SHA-256, SHA-384 | - |
| Key Derivation | HKDF | SHA-256 |
| MAC | HMAC-SHA-256 | 256-bit key |

### 3.2 Deprecated Algorithms

The following MUST NOT be used:

- RSA (for new deployments)
- SHA-1
- MD5
- DES, 3DES
- RC4
- CBC mode encryption

### 3.3 Cryptographic Agility

The protocol supports algorithm negotiation for future-proofing:

```protobuf
message CryptoCapabilities {
    repeated string supported_signature_algorithms = 1;
    repeated string supported_encryption_algorithms = 2;
    repeated string supported_hash_algorithms = 3;
}

// Negotiation result
message NegotiatedCrypto {
    string signature_algorithm = 1;  // e.g., "Ed25519"
    string encryption_algorithm = 2; // e.g., "AES-256-GCM"
    string hash_algorithm = 3;       // e.g., "SHA-256"
}
```

---

## 4. Transport Security

### 4.1 TLS Configuration

**Required TLS Version:** 1.3 only

**Cipher Suites (in preference order):**

```
TLS_AES_256_GCM_SHA384
TLS_CHACHA20_POLY1305_SHA256
TLS_AES_128_GCM_SHA256
```

### 4.2 TLS Configuration Example

```yaml
# Coordinator TLS configuration
tls:
  min_version: "1.3"
  max_version: "1.3"

  cipher_suites:
    - TLS_AES_256_GCM_SHA384
    - TLS_CHACHA20_POLY1305_SHA256

  client_auth: required

  certificate: /etc/atomicsettle/server.crt
  private_key: /etc/atomicsettle/server.key
  client_ca: /etc/atomicsettle/participant-ca.crt

  # OCSP stapling
  ocsp_stapling: true

  # Session tickets disabled (forward secrecy)
  session_tickets: false
```

### 4.3 Certificate Pinning

Participants SHOULD pin coordinator certificates:

```python
class CertificatePinner:
    def __init__(self, pinned_certs: List[str]):
        # Store SHA-256 hashes of pinned certificate public keys
        self.pinned_hashes = set(pinned_certs)

    def verify(self, cert: X509Certificate) -> bool:
        cert_hash = sha256(cert.public_key_der()).hex()
        return cert_hash in self.pinned_hashes
```

---

## 5. Message Security

### 5.1 Message Signing

All protocol messages MUST be signed by the sender:

```python
def sign_message(message: Message, private_key: Ed25519PrivateKey) -> SignedMessage:
    # 1. Serialize message (canonical form)
    canonical = canonicalize(message)

    # 2. Create signature
    signature = private_key.sign(canonical)

    # 3. Attach signature
    return SignedMessage(
        message=message,
        signature=Signature(
            algorithm="Ed25519",
            value=signature,
            key_id=get_key_id(private_key)
        )
    )

def verify_signature(signed_message: SignedMessage, public_key: Ed25519PublicKey) -> bool:
    canonical = canonicalize(signed_message.message)
    return public_key.verify(signed_message.signature.value, canonical)
```

### 5.2 Canonicalization

To ensure consistent signatures, messages are canonicalized before signing:

```python
def canonicalize(message: Message) -> bytes:
    """
    Produce canonical byte representation for signing.

    Rules:
    1. Serialize to protobuf binary format
    2. Fields in tag order (protobuf default)
    3. No unknown fields
    4. Default values omitted
    """
    # Remove signature field before canonicalization
    message_copy = message.copy()
    message_copy.signature = None

    return message_copy.SerializeToString(deterministic=True)
```

### 5.3 Message Encryption

Settlement details are encrypted for confidentiality:

```python
def encrypt_settlement_details(
    details: SettlementDetails,
    recipient_public_key: X25519PublicKey,
    sender_private_key: X25519PrivateKey
) -> EncryptedPayload:
    # 1. Derive shared secret (X25519)
    shared_secret = x25519(sender_private_key, recipient_public_key)

    # 2. Derive encryption key (HKDF)
    encryption_key = hkdf(
        shared_secret,
        salt=random_bytes(32),
        info=b"atomicsettle-message-encryption",
        length=32
    )

    # 3. Encrypt with AES-256-GCM
    nonce = random_bytes(12)
    ciphertext, tag = aes_gcm_encrypt(
        key=encryption_key,
        nonce=nonce,
        plaintext=details.SerializeToString(),
        associated_data=b"settlement-details"
    )

    return EncryptedPayload(
        algorithm="X25519-AES-256-GCM",
        ephemeral_public_key=sender_private_key.public_key(),
        nonce=nonce,
        ciphertext=ciphertext,
        tag=tag
    )
```

### 5.4 Replay Protection

Messages include timestamp and sequence numbers:

```python
def validate_message_freshness(message: Message) -> bool:
    # 1. Check timestamp within acceptable window (5 minutes)
    message_time = message.envelope.timestamp
    if abs(now() - message_time) > timedelta(minutes=5):
        raise MessageTooOld()

    # 2. Check sequence number (per participant)
    expected_seq = get_expected_sequence(message.sender)
    if message.sequence <= expected_seq:
        raise ReplayDetected()

    # 3. Update expected sequence
    update_expected_sequence(message.sender, message.sequence)

    return True
```

---

## 6. Key Management

### 6.1 Key Types

| Key Type | Algorithm | Usage | Storage |
|----------|-----------|-------|---------|
| Signing Key | Ed25519 | Message signatures | HSM |
| TLS Key | ECDSA P-256 | TLS authentication | HSM |
| Encryption Key | X25519 | Message encryption | HSM |
| Master Key | AES-256 | Key encryption | HSM (root) |

### 6.2 Key Storage

**Hardware Security Module (HSM) Requirements:**

- FIPS 140-2 Level 3 or higher
- Support for Ed25519 and ECDSA
- Key generation within HSM
- Audit logging of all key operations

**Key Hierarchy:**

```
┌────────────────────────────────────────┐
│            Master Key (HSM)            │
│  Never leaves HSM, wraps other keys    │
└─────────────────┬──────────────────────┘
                  │
    ┌─────────────┼─────────────┐
    │             │             │
    ▼             ▼             ▼
┌────────┐  ┌────────┐   ┌────────────┐
│Signing │  │  TLS   │   │ Encryption │
│  Key   │  │  Key   │   │    Key     │
└────────┘  └────────┘   └────────────┘
```

### 6.3 Key Rotation

| Key Type | Rotation Period | Overlap Period |
|----------|-----------------|----------------|
| Signing Key | 90 days | 7 days |
| TLS Certificate | 365 days | 30 days |
| Encryption Key | 30 days | 7 days |

**Rotation Procedure:**

```python
async def rotate_signing_key(participant: Participant):
    # 1. Generate new key in HSM
    new_key = await hsm.generate_ed25519_key()

    # 2. Register new key with coordinator
    await coordinator.register_key(
        participant_id=participant.id,
        public_key=new_key.public_key,
        valid_from=now() + timedelta(days=1),
        valid_until=now() + timedelta(days=90)
    )

    # 3. Begin overlap period (both keys valid)
    # 4. After 7 days, old key becomes verify-only
    # 5. After 14 days, old key is deactivated
```

### 6.4 Key Compromise Response

```python
async def handle_key_compromise(participant_id: str, compromised_key_id: str):
    # 1. Immediately revoke compromised key
    await coordinator.revoke_key(participant_id, compromised_key_id)

    # 2. Reject all pending settlements involving participant
    await coordinator.reject_pending_settlements(participant_id)

    # 3. Disconnect participant
    await coordinator.disconnect_participant(participant_id)

    # 4. Generate incident report
    incident = await create_incident_report(
        type="KEY_COMPROMISE",
        participant_id=participant_id,
        key_id=compromised_key_id
    )

    # 5. Notify all participants
    await coordinator.broadcast_security_alert(incident)

    # 6. Participant must complete re-onboarding with new keys
```

---

## 7. Access Control

### 7.1 Role-Based Access Control

| Role | Permissions |
|------|-------------|
| Participant | Send settlements, view own settlements, query balance |
| Coordinator Operator | View all settlements, manage participants, view metrics |
| Security Admin | Key management, audit log access, incident response |
| System Admin | Infrastructure management (no settlement access) |

### 7.2 Settlement Authorization

```python
def authorize_settlement(request: SettleRequest, sender: Participant) -> AuthzResult:
    # 1. Verify sender matches certificate
    if request.sender != sender.id:
        return AuthzResult.DENIED("Sender mismatch")

    # 2. Verify sender has settlement permission
    if not sender.has_permission("SEND_SETTLEMENT"):
        return AuthzResult.DENIED("No settlement permission")

    # 3. Verify amount within sender's limits
    if request.amount > sender.settlement_limit:
        return AuthzResult.DENIED("Amount exceeds limit")

    # 4. Verify currency allowed for sender
    if request.currency not in sender.allowed_currencies:
        return AuthzResult.DENIED("Currency not allowed")

    # 5. Verify receiver accepts from sender
    receiver = get_participant(request.receiver)
    if sender.id in receiver.blocked_counterparties:
        return AuthzResult.DENIED("Blocked by receiver")

    return AuthzResult.ALLOWED
```

### 7.3 API Rate Limiting

```python
class RateLimiter:
    LIMITS = {
        "settlement_request": (1000, 60),    # 1000 per minute
        "balance_query": (100, 60),          # 100 per minute
        "connection": (10, 60),              # 10 per minute
    }

    async def check_limit(self, participant_id: str, action: str) -> bool:
        limit, window = self.LIMITS.get(action, (100, 60))
        key = f"ratelimit:{participant_id}:{action}"

        current = await redis.incr(key)
        if current == 1:
            await redis.expire(key, window)

        return current <= limit
```

---

## 8. Audit and Logging

### 8.1 Audit Events

All security-relevant events MUST be logged:

| Event Type | Required Fields |
|------------|-----------------|
| AUTHENTICATION_SUCCESS | participant_id, certificate_fingerprint, timestamp |
| AUTHENTICATION_FAILURE | participant_id, reason, source_ip, timestamp |
| SETTLEMENT_INITIATED | settlement_id, sender, receiver, amount, timestamp |
| SETTLEMENT_COMPLETED | settlement_id, status, duration, timestamp |
| KEY_OPERATION | operation, key_id, participant_id, timestamp |
| ACCESS_DENIED | participant_id, action, reason, timestamp |
| CONFIGURATION_CHANGE | admin_id, change_type, before, after, timestamp |

### 8.2 Audit Log Format

```json
{
  "timestamp": "2026-01-17T10:30:00.123456Z",
  "event_type": "SETTLEMENT_INITIATED",
  "event_id": "550e8400-e29b-41d4-a716-446655440000",
  "participant_id": "JPMORGAN_NY",
  "settlement_id": "019456ab-1234-7def-8901-234567890abc",
  "details": {
    "sender": "JPMORGAN_NY",
    "receiver": "HSBC_LONDON",
    "amount": "1000000.00",
    "currency": "USD"
  },
  "source_ip": "10.0.1.50",
  "certificate_fingerprint": "sha256:abc123...",
  "coordinator_node": "coordinator-1"
}
```

### 8.3 Log Integrity

Audit logs are protected against tampering:

```python
class SecureAuditLog:
    def __init__(self, signing_key: Ed25519PrivateKey):
        self.signing_key = signing_key
        self.hash_chain = None

    def append(self, event: AuditEvent) -> None:
        # 1. Add sequence number
        event.sequence = self.get_next_sequence()

        # 2. Link to previous entry (hash chain)
        event.previous_hash = self.hash_chain

        # 3. Compute hash of this entry
        event_hash = sha256(event.serialize())
        self.hash_chain = event_hash

        # 4. Sign the entry
        event.signature = self.signing_key.sign(event_hash)

        # 5. Write to append-only storage
        self.storage.append(event)
```

### 8.4 Log Retention

| Log Type | Retention Period | Archive |
|----------|------------------|---------|
| Security Events | 7 years | Yes |
| Settlement Events | 10 years | Yes |
| Debug/Trace | 30 days | No |
| Metrics | 2 years | Yes |

---

## 9. Threat Model

### 9.1 Threat Actors

| Actor | Capability | Motivation |
|-------|------------|------------|
| External Attacker | Network access, no credentials | Financial gain, disruption |
| Compromised Participant | Valid credentials, limited access | Fraud, data theft |
| Malicious Insider | Elevated access | Fraud, sabotage |
| Nation State | Advanced persistent threat | Espionage, disruption |

### 9.2 Attack Vectors and Mitigations

| Attack | Mitigation |
|--------|------------|
| Man-in-the-middle | mTLS, certificate pinning |
| Replay attack | Timestamps, sequence numbers |
| Message tampering | Digital signatures |
| Unauthorized access | mTLS authentication, RBAC |
| DoS/DDoS | Rate limiting, connection limits |
| SQL injection | Parameterized queries, input validation |
| Key theft | HSM storage, key rotation |
| Insider threat | Separation of duties, audit logging |

### 9.3 Security Assumptions

1. HSMs are not compromised
2. PKI root CA is secure and offline
3. Participants secure their own private keys
4. Network infrastructure (firewalls, load balancers) is correctly configured
5. Operating systems and dependencies are patched

### 9.4 Trust Boundaries

```
┌───────────────────────────────────────────────────────────────┐
│                    TRUSTED ZONE (Coordinator)                  │
│                                                               │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐       │
│  │ Coordinator │◄──►│   Ledger    │◄──►│     HSM     │       │
│  │   Process   │    │   Database  │    │             │       │
│  └──────┬──────┘    └─────────────┘    └─────────────┘       │
│         │                                                     │
└─────────┼─────────────────────────────────────────────────────┘
          │ TLS + mTLS
          │
┌─────────▼─────────────────────────────────────────────────────┐
│                    UNTRUSTED ZONE (Network)                   │
└─────────┬─────────────────────────────────────────────────────┘
          │
┌─────────▼─────────────────────────────────────────────────────┐
│                 PARTICIPANT ZONE (Bank Network)               │
│                                                               │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐       │
│  │ Participant │◄──►│    Core     │◄──►│     HSM     │       │
│  │   Adapter   │    │   Banking   │    │             │       │
│  └─────────────┘    └─────────────┘    └─────────────┘       │
│                                                               │
└───────────────────────────────────────────────────────────────┘
```

---

## 10. Incident Response

### 10.1 Security Incident Classification

| Severity | Description | Response Time |
|----------|-------------|---------------|
| Critical | Active breach, key compromise | Immediate |
| High | Attempted breach, vulnerability discovered | 1 hour |
| Medium | Policy violation, suspicious activity | 4 hours |
| Low | Informational, minor policy deviation | 24 hours |

### 10.2 Incident Response Procedure

```
1. DETECT
   - Security monitoring alerts
   - Participant reports
   - Audit log analysis

2. CONTAIN
   - Isolate affected systems
   - Revoke compromised credentials
   - Block malicious IPs

3. ERADICATE
   - Remove malicious access
   - Patch vulnerabilities
   - Rotate affected keys

4. RECOVER
   - Restore from clean backups
   - Verify system integrity
   - Resume operations

5. POST-INCIDENT
   - Root cause analysis
   - Update procedures
   - Notify stakeholders
```

### 10.3 Communication During Incidents

```python
class IncidentNotifier:
    def notify_critical_incident(self, incident: SecurityIncident):
        # 1. Notify all participants
        for participant in self.get_all_participants():
            self.send_security_alert(participant, incident)

        # 2. Notify regulators (if required)
        if incident.requires_regulatory_notification():
            self.notify_regulators(incident)

        # 3. Update public status page
        self.update_status_page(
            status="DEGRADED",
            message="Security incident under investigation"
        )
```

---

## Appendix A: Security Checklist

### Pre-Production Security Review

- [ ] All TLS configurations use TLS 1.3 only
- [ ] All keys stored in HSM
- [ ] Certificate chain validated
- [ ] Certificate revocation checking enabled
- [ ] Rate limiting configured
- [ ] Audit logging enabled
- [ ] Log integrity protection enabled
- [ ] Penetration testing completed
- [ ] Security review by external auditor
- [ ] Incident response plan documented
- [ ] Key rotation procedures tested

---

## Appendix B: Change Log

| Version | Date | Changes |
|---------|------|---------|
| 0.1.0 | 2026-01-17 | Initial draft |
