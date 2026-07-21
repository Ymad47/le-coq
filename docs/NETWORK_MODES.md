# Modes Réseau dans Le Coq : Clair vs Furtif

## Objectif
Permettre à l'utilisateur de choisir entre deux modes de requêtes réseau :
- **Clair** : rapide, direct, sans anonymisation
- **Furtif** : anonyme via Tor, plus lent mais sécurisé

## Architecture API

### 1. Nouvel endpoint global
```http
POST /api/v1/network-mode
{
  "mode": "furtif"  ← ou "claire"
}
```

### 2. Paramètre par requête
```http
POST /api/v1/ingest
{
  "source": "opencorporates",
  "mode": "furtif",   ← Optionnel (par défaut: claire)
  "query": "Acme Corp"
}
```

### 3. En-tête HTTP (alternative)
```http
X-Network-Mode: furtif
```

---

## Backend (Rust)

### Gestion du mode réseau

```rust
enum NetworkMode {
    Clair,
    Furtif,
}

impl NetworkMode {
    fn from_str(s: &str) -> Self {
        match s {
            "furtif" => NetworkMode::Furtif,
            _ => NetworkMode::Clair,
        }
    }
}
```

### Client HTTP adaptable

```rust
use reqwest;

fn build_client(mode: NetworkMode) -> reqwest::Result<reqwest::Client> {
    let builder = reqwest::Client::builder();

    match mode {
        NetworkMode::Furtif => builder.proxy(
            reqwest::Proxy::all("socks5://127.0.0.1:9050")?
        ),
        NetworkMode::Clair => builder,
    }
    .build()
}
```

---

## Plugins Python

### Adaptation des plugins

```python
import socks
import socket
import requests

def set_network_mode(mode="claire"):
    if mode == "furtif":
        socks.set_default_proxy(socks.SOCKS5, "127.0.0.1", 9050)
        socket.socket = socks.socksocket
    else:
        socket.socket = socket._socketobject if hasattr(socket, '_socketobject') else socket.socket

def fetch_data(url, mode="claire"):
    set_network_mode(mode)
    return requests.get(url).json()
```

---

## Frontend (Svelte)

### Toggle dans l'interface

```svelte
<script>
  let networkMode = "claire";

  function toggleMode() {
    networkMode = networkMode === "claire" ? "furtif" : "claire";
    fetch("/api/v1/network-mode", {
      method: "POST",
      body: JSON.stringify({ mode: networkMode })
    });
  }
</script>

<div class="mode-toggle">
  <button on:click={toggleMode}>
    Mode: {networkMode === "claire" ? "🔍 Clair" : "🧅 Furtif"}
  </button>
</div>
```

---

## 🔐 Sécurité & Bonnes pratiques

### 1. Mode par défaut
- **Clair** pour performance
- **Furtif** pour sensibilité

### 2. Journalisation
- Loguer le mode utilisé pour chaque requête
- Pas de trace IP utilisateur

### 3. Cache
- Séparer les caches par mode
- Éviter fuite entre modes

### 4. Chiffrement des données
- PostgreSQL avec chiffrement TDE (Transparent Data Encryption)
- Clés de chiffrement gérées localement
- Backup chiffré (optionnel)

⚠️ **À venir** : Chiffrement systématique des données sensibles (personnes, lieux, événements) dans un futur proche pour prévenir tout risque de fuite.

---

## Test des modes

### Via curl
```bash
# Mode clair
curl -X POST http://localhost:8000/api/v1/ingest \
  -H "Content-Type: application/json" \
  -d '{"source": "rss", "query": "news", "mode": "claire"}'

# Mode furtif
curl -X POST http://localhost:8000/api/v1/ingest \
  -H "Content-Type: application/json" \
  -d '{"source": "rss", "query": "news", "mode": "furtif"}'
```

---

## Documentation utilisateur

### Quand utiliser quel mode ?
| Cas d'usage | Mode recommandé |
|-------------|-----------------|
| Exploration rapide | Clair |
| Sources sensibles | Furtif |
| Tests et dev | Clair |
| Investigations critiques | Furtif |

---