from setuptools import setup, find_packages

setup(
    name="le-coq-plugins",
    version="0.1.0",
    description="Le Coq OSINT plugin system and data sources",
    license="AGPL-3.0",
    python_requires=">=3.11",
    packages=find_packages(),
    install_requires=[
        "requests>=2.31",
        "beautifulsoup4>=4.12",
        "aiohttp>=3.9",
        "pydantic>=2.5",
        "spacy>=3.7",
        "sentence-transformers>=2.2",
    ],
    extras_require={
        "dev": [
            "pytest>=7.4",
            "pytest-asyncio>=0.21",
            "black>=23.12",
            "ruff>=0.1",
        ],
    },
)