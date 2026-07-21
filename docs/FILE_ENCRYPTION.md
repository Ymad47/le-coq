# Chiffrement des Fichiers de Dev dans Le Coq

## Objectif
Renforcer la sécurité de chaque stack en chiffrant les fichiers sensibles (configurations, tokens, données locales) pour prévenir tout risque de fuite.

---

## 1. Backend Rust

### Fichiers concernés
- `.env`
- `config.yaml`
- Fichiers de logs sensibles

### Outils
- `age` (simple, moderne)
- `gpg` (standard, compatible)

### Exemple avec `age`
```bash
# Chiffrer
age -p config.yaml > config.yaml.age

# Déchiffrer
age -d -i ~/.age/key.txt config.yaml.age > config.yaml
```

### Git ignore
```
*.age
.env
config.yaml
```

---

## 2. Base de données (PostgreSQL)

### Chiffrement au repos
- Extension `pg_tde` (expérimental)
- OU chiffrement système avec LUKS

### Sauvegardes
```bash
# Chiffrer backup
age -p backup.sql > backup.sql.age
```

---

## 3. Plugins Python

### Fichiers sensibles
- Tokens API
- Fichiers de cache contenant des données privées

### Outils
- `cryptography` (Fernet)

### Exemple
```python
from cryptography.fernet import Fernet

# Générer une clé
key = Fernet.generate_key()
cipher = Fernet(key)

# Chiffrer
encrypted = cipher.encrypt(b"API_KEY=***")

# Déchiffrer
decrypted = cipher.decrypt(encrypted)
```

---

## 4. Frontend Svelte

### Variables d’environnement
- `.env` chiffré avec `age` ou `sops`

### Stockage local
- IndexedDB chiffré (si utilisé)

---

## 5. CLI Zig

### Fichiers de config
- Chiffrés avec `age` ou `sops`

### Exports sensibles
- Chiffrés avant écriture sur disque

---

## Outils recommandés

| Outil | Usage | Commande exemple |
|-------|-------|------------------|
| `age` | Chiffrement simple | `age -p file > file.age` |
| `gpg` | Chiffrement + signature | `gpg --encrypt file` |
| `sops` | Chiffrement YAML/JSON | `sops -e config.yaml` |
| `LUKS` | Chiffrement disque | `cryptsetup luksFormat /dev/sdX` |
| `Fernet` | Chiffrement Python | `cipher.encrypt(data)` |

---

## Bonnes pratiques

- 🔐 **Clés locales** : jamais versionnées
- 📁 **Git ignore** : tous les fichiers sensibles
- 🔁 **Rotation** : clés régénérées tous les 6 mois
- 🧾 **Documentation** : guide dans `docs/SECURITY.md`

---

## À venir

- Script global de chiffrement/déchiffrement
- Intégration dans le CLI Zig
- Support `sops` pour toutes les configs

---