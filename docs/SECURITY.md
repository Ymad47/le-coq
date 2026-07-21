# Le Coq — Security Guidelines

**Sécurité API & Infrastructure — Phase 1 à Phase 3**

---

## 🔒 Authentification API

### Phase 1 (MVP)

**API Keys — Simple et efficace**

```rust
// backend/src/auth.rs
pub struct ApiKey {
    pub id: Uuid,
    pub key: String,  // SHA-256 hash
    pub tenant_id: Uuid,
    pub permissions: Vec<String>,  // ["read", "write", "admin"]
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

// Middleware axum
async fn auth_middleware(
    headers: HeaderMap,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = headers
        .get("X-API-Key")
        .ok_or(StatusCode::UNAUTHORIZED)?;
    
    let api_key = validate_api_key(auth_header.to_str().unwrap())
        .await
        .ok_or(StatusCode::UNAUTHORIZED)?;
    
    // Inject tenant_id in request extensions
    Ok(next.run(api_key.into_request()).await)
}
```

**Usage client :**
```bash
curl -H "X-API-Key: your-secret-key" https://api.lecoq.fr/api/v1/entities
```

---

### Phase 2 (SaaS)

**JWT + Refresh Tokens**

```rust
pub struct JwtClaims {
    pub sub: String,      // user_id
    pub tenant_id: String,
    pub permissions: Vec<String>,
    pub exp: i64,         // expiration
    pub iat: i64,         // issued at
}

// Endpoints
POST /api/v1/auth/login      // email/password → JWT + refresh
POST /api/v1/auth/refresh    // refresh token → new JWT
POST /api/v1/auth/logout     // invalidate refresh token
```

---

## 🛡️ Rate Limiting

### Phase 1

**Simple in-memory (Redis)**

```rust
// backend/src/rate_limit.rs
use redis::Client;

pub async fn check_rate_limit(
    api_key: &str,
    endpoint: &str,
    limit: u32,        // requests
    window: u32,       // seconds
) -> Result<bool, Error> {
    let key = format!("ratelimit:{}:{}", api_key, endpoint);
    let count = redis::incr(&key, 1).await?;
    
    if count == 1 {
        redis::expire(&key, window).await?;
    }
    
    Ok(count <= limit)
}

// Usage in middleware
// Limit: 100 req/min for read, 10 req/min for write
```

**Headers de réponse :**
```
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 95
X-RateLimit-Reset: 1626940800
```

---

### Phase 2

**Tier-based limits**

| Tier | Read/min | Write/min | Burst |
|------|----------|-----------|-------|
| Community | 60 | 5 | 10 |
| Premium | 600 | 60 | 100 |
| Enterprise | 6000 | 600 | 1000 |

---

## 🔐 TLS / HTTPS

### Obligatoire (Dès Phase 1)

**Certbot + Let's Encrypt**

```bash
# Hetzner server
apt install certbot python3-certbot-nginx

certbot certonly --standalone \
  -d api.lecoq.fr \
  -d www.lecoq.fr \
  --email ymad@ymad.fr \
  --agree-tos \
  --non-interactive

# Auto-renewal (cron)
0 0 1 * * certbot renew --quiet
```

**Nginx reverse proxy :**
```nginx
server {
    listen 443 ssl http2;
    server_name api.lecoq.fr;
    
    ssl_certificate /etc/letsencrypt/live/api.lecoq.fr/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/api.lecoq.fr/privkey.pem;
    
    # Modern SSL config
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256;
    ssl_prefer_server_ciphers off;
    
    # Security headers
    add_header Strict-Transport-Security "max-age=63072000" always;
    add_header X-Frame-Options DENY;
    add_header X-Content-Type-Options nosniff;
    add_header X-XSS-Protection "1; mode=block";
    add_header Content-Security-Policy "default-src 'self'";
    
    location / {
        proxy_pass http://localhost:8080;  # Rust backend
    }
}

# Redirect HTTP → HTTPS
server {
    listen 80;
    server_name api.lecoq.fr;
    return 301 https://$server_name$request_uri;
}
```

---

## 📊 Audit Logs

### Phase 2 (Requis pour Enterprise + Dark Web)

**Schema PostgreSQL :**
```sql
CREATE TABLE audit_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    user_id UUID,
    api_key_id UUID,
    action TEXT NOT NULL,          -- "entity.read", "ingest.create", etc.
    resource_type TEXT,            -- "entity", "relationship", "source"
    resource_id UUID,
    request_method TEXT,           -- GET, POST, PUT, DELETE
    request_path TEXT,
    request_body JSONB,            -- Redacted (no secrets)
    response_status INTEGER,
    response_time_ms INTEGER,
    ip_address INET,
    user_agent TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX ON audit_logs (tenant_id, created_at DESC);
CREATE INDEX ON audit_logs (user_id, created_at DESC);
CREATE INDEX ON audit_logs (action, created_at DESC);
```

**Middleware logging :**
```rust
async fn audit_log_middleware(
    api_key: ApiKey,
    request: Request<Body>,
    next: Next,
) -> Response {
    let start = Instant::now();
    let path = request.uri().path().to_string();
    let method = request.method().to_string();
    
    let response = next.run(request).await;
    
    let duration = start.elapsed().as_millis() as i32;
    
    // Log to PostgreSQL (async)
    tokio::spawn(async move {
        insert_audit_log(AuditLog {
            tenant_id: api_key.tenant_id,
            api_key_id: api_key.id,
            action: format!("{}.{}", method, path),
            response_status: response.status().as_u16() as i32,
            response_time_ms: duration,
            // ...
        }).await;
    });
    
    response
}
```

---

## 🧅 Dark Web — Security Isolation

### Phase 3 (Module Tor)

**Architecture isolée :**
```
┌─────────────────────────────────────────────┐
│  Tor Plugin Container (isolated network)    │
│  ┌─────────────┐                            │
│  │ Python      │                            │
│  │ stem lib    │─── socks5 ───> Tor         │
│  │ scraper     │                            │
│  └─────────────┘                            │
│         │                                   │
│         │ NATS (publish only)               │
└─────────┼───────────────────────────────────┘
          │
          ▼
┌─────────────────────────────────────────────┐
│  Backend Container (main network)           │
│  ┌─────────────┐                            │
│  │ Rust        │                            │
│  │ axum        │<─── NATS (consume)         │
│  │ API         │                            │
│  └─────────────┘                            │
└─────────────────────────────────────────────┘
```

**Docker network isolation :**
```yaml
# docker-compose.yml
services:
  tor-plugin:
    image: lecoq-tor-plugin
    networks:
      - tor_network  # Isolated, no access to backend network
    environment:
      - TOR_SOCKS_PORT=9050
      - NATS_URL=nats://nats:4222
    volumes:
      - ./tor-data:/data  # Persistent Tor state
  
  backend:
    image: lecoq-backend
    networks:
      - main_network
    # No access to tor_network

  nats:
    image: nats:latest
    networks:
      - tor_network
      - main_network  # Bridge between networks
```

**Security rules :**
- ✅ Read-only (no POST/interaction)
- ✅ Tor circuit rotation (every request)
- ✅ No credentials stored
- ✅ Audit logging (qui a scrapé quoi, quand)
- ✅ Container isolation (pas d'accès direct au backend)
- ❌ Jamais de login sur les services darknet
- ❌ Jamais de téléchargement de contenu illégal

---

## 🔑 Secrets Management

### Phase 1 (Fichiers .env)

```bash
# .env (non versionné, chmod 600)
DATABASE_URL=postgresql://lecoq:***@localhost:5432/lecoq
REDIS_URL=redis://localhost:6379
NATS_URL=nats://localhost:4222
API_KEY_SECRET=super-secret-key-min-32-chars
JWT_SECRET=another-secret-for-jwt-tokens
MEILISEARCH_KEY=master-key-from-install
```

**Permissions :**
```bash
chmod 600 .env
chown yaya:yaya .env
```

---

### Phase 2 (HashiCorp Vault)

```bash
# Vault server (Hetzner VM dédiée)
vault server -dev -dev-root-token-id="root"

# Secrets
vault kv put secret/lecoq/database \
  url="postgresql://..." \
  username="lecoq" \
  password="***"

vault kv put secret/lecoq/api-keys \
  master="***"

# Backend reads from Vault
export VAULT_ADDR="http://vault:8200"
export VAULT_TOKEN="***"
```

---

## 🚨 Security Headers (Frontend)

### Content Security Policy (CSP)

```html
<!-- frontend/index.html -->
<meta http-equiv="Content-Security-Policy" 
      content="default-src 'self'; 
               script-src 'self' 'unsafe-inline'; 
               style-src 'self' 'unsafe-inline'; 
               img-src 'self' data: https:; 
               connect-src 'self' https://api.lecoq.fr;">
```

### Headers Nginx

```nginx
# Déjà dans config TLS ci-dessus
add_header Strict-Transport-Security "max-age=63072000" always;
add_header X-Frame-Options DENY;
add_header X-Content-Type-Options nosniff;
add_header X-XSS-Protection "1; mode=block";
add_header Referrer-Policy "strict-origin-when-cross-origin";
add_header Permissions-Policy "geolocation=(), microphone=(), camera=()";
```

---

## 📈 Security Checklist par Phase

### Phase 1 (MVP)

- [ ] **TLS/HTTPS** (Certbot + auto-renewal)
- [ ] **API Keys** (hash SHA-256, stocké en DB)
- [ ] **Rate Limiting** (Redis, 100 req/min)
- [ ] **CORS** (whitelist domains autorisés)
- [ ] **Security Headers** (HSTS, X-Frame-Options, etc.)
- [ ] **Input Validation** (serde validators, regex)
- [ ] **SQL Injection** (sqlx prepared statements)
- [ ] **Logs** (fichier, rotation weekly)
- [ ] **Backup** (pg_dump daily, Storage Box)

### Phase 2 (SaaS)

- [ ] **JWT Auth** (access + refresh tokens)
- [ ] **Row-Level Security** (PostgreSQL RLS)
- [ ] **Audit Logs** (qui a fait quoi, quand)
- [ ] **Rate Limiting Tier-based** (Community/Premium/Enterprise)
- [ ] **Monitoring** (Prometheus + Grafana)
- [ ] **Alerts** (Slack/email on errors > threshold)
- [ ] **Vault** (secrets management)
- [ ] **CI/CD Security** (SAST, dependency scan)

### Phase 3 (Enterprise + Dark Web)

- [ ] **OAuth2/SAML** (SSO enterprise)
- [ ] **2FA/MFA** (TOTP, WebAuthn)
- [ ] **Tor Isolation** (container/network separation)
- [ ] **Audit Logs Compliance** (GDPR, SOC2)
- [ ] **Penetration Testing** (annual)
- [ ] **Security Certifications** (ISO27001, SOC2)
- [ ] **Incident Response Plan** (playbook)
- [ ] **Data Encryption at Rest** (TDE PostgreSQL)

---

## 🧪 Security Testing

### Phase 1

**OWASP ZAP (gratuit) :**
```bash
# Scan automatique
docker run -t owasp/zap2docker-stable zap-baseline.py \
  -t https://api.lecoq.fr

# Scan complet
docker run -t owasp/zap2docker-stable zap-full-scan.py \
  -t https://api.lecoq.fr \
  -r report.html
```

**Dependency Audit :**
```bash
# Rust
cargo audit

# Python
pip install safety
safety check -r requirements.txt

# Node.js
npm audit
```

---

### Phase 2

**SAST (Static Application Security Testing) :**
```yaml
# .github/workflows/security.yml
name: Security Scan
on: [push, pull_request]

jobs:
  sast:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Rust Audit
        run: cargo audit
      
      - name: Python Safety
        run: |
          pip install safety
          safety check -r requirements.txt
      
      - name: Semgrep (multi-language)
        run: |
          pip install semgrep
          semgrep --config auto .
```

---

## 🚨 Incident Response

### Playbook — Data Breach

1. **Detect** (alerts, logs, user report)
2. **Contain** (revoke API keys, isolate affected tenant)
3. **Assess** (what data, how many users, root cause)
4. **Notify** (users affected, authorities if GDPR)
5. **Remediate** (fix vulnerability, rotate secrets)
6. **Review** (post-mortem, update playbook)

**Template notification :**
```
Subject: [Le Coq] Security Incident — Action Required

Dear [User],

We detected unauthorized access to [data type] on [date].

What happened:
- [Brief description]

What data was affected:
- [List of data types]

What we did:
- [Containment actions]

What you should do:
- [Rotate API key / Change password / etc.]

We apologize for this incident and are committed to transparency.

— Le Coq Security Team
```

---

## 📚 References

- [OWASP API Security Top 10](https://owasp.org/www-project-api-security/)
- [Rust Security Guidelines](https://rust-lang.github.io/rust-clippy/master/index.html)
- [PostgreSQL Security Best Practices](https://wiki.postgresql.org/wiki/Security_Best_Practices)
- [GDPR Compliance Checklist](https://gdpr.eu/checklist/)

---

*Fabriqué en France 🇫🇷*  
*Souverain. Transparent. Puissant.*
