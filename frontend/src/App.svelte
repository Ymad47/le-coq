<script lang="ts">
  import { onMount } from 'svelte';
  
  let entities: any[] = [];
  let query = '';
  let loading = false;
  
  async function search() {
    if (!query.trim()) return;
    
    loading = true;
    try {
      const resp = await fetch(`/api/v1/entities?q=${encodeURIComponent(query)}`);
      if (resp.ok) {
        entities = await resp.json();
      }
    } catch (e) {
      console.error('Search failed:', e);
    }
    loading = false;
  }
  
  function handleKey(e: KeyboardEvent) {
    if (e.key === 'Enter') search();
  }
</script>

<main>
  <h1>🐓 Le Coq</h1>
  <p class="subtitle">Geopolitical Intelligence Platform</p>
  
  <div class="search-box">
    <input 
      type="text" 
      bind:value={query} 
      on:keydown={handleKey}
      placeholder="Search entities, relationships, events..."
    />
    <button on:click={search} disabled={loading}>
      {loading ? 'Searching...' : 'Search'}
    </button>
  </div>
  
  {#if entities.length > 0}
    <div class="results">
      <h2>Results ({entities.length})</h2>
      <ul>
        {#each entities as entity}
          <li>
            <strong>{entity.name}</strong>
            <span class="type">{entity.type}</span>
            <p>{entity.properties?.address || entity.properties?.country || ''}</p>
          </li>
        {/each}
      </ul>
    </div>
  {/if}
</main>

<style>
  :global(body) {
    margin: 0;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    background: #0f172a;
    color: #e2e8f0;
  }
  
  main {
    max-width: 900px;
    margin: 0 auto;
    padding: 2rem;
  }
  
  h1 {
    font-size: 2.5rem;
    margin-bottom: 0.5rem;
    background: linear-gradient(135deg, #f59e0b, #ef4444);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
  }
  
  .subtitle {
    color: #94a3b8;
    margin-bottom: 2rem;
  }
  
  .search-box {
    display: flex;
    gap: 1rem;
    margin-bottom: 2rem;
  }
  
  input {
    flex: 1;
    padding: 0.75rem 1rem;
    font-size: 1rem;
    border: 1px solid #334155;
    border-radius: 6px;
    background: #1e293b;
    color: #e2e8f0;
  }
  
  input:focus {
    outline: none;
    border-color: #f59e0b;
  }
  
  button {
    padding: 0.75rem 1.5rem;
    font-size: 1rem;
    font-weight: 600;
    border: none;
    border-radius: 6px;
    background: #f59e0b;
    color: #0f172a;
    cursor: pointer;
  }
  
  button:hover {
    background: #fbbf24;
  }
  
  button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  
  .results {
    background: #1e293b;
    border-radius: 8px;
    padding: 1.5rem;
  }
  
  h2 {
    margin-top: 0;
    font-size: 1.25rem;
  }
  
  ul {
    list-style: none;
    padding: 0;
  }
  
  li {
    padding: 1rem 0;
    border-bottom: 1px solid #334155;
  }
  
  li:last-child {
    border-bottom: none;
  }
  
  .type {
    display: inline-block;
    padding: 0.25rem 0.5rem;
    font-size: 0.75rem;
    border-radius: 4px;
    background: #334155;
    color: #94a3b8;
    margin-left: 0.5rem;
  }
  
  p {
    margin: 0.5rem 0 0;
    color: #64748b;
    font-size: 0.875rem;
  }
</style>
