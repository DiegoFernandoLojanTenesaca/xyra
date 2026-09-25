<script lang="ts">
  import { Check } from '@lucide/svelte';
  import { app } from '../../app.svelte';

  const t = $derived(app.t);
  const rules = $derived(Object.values(t('settings:securityRules', { returnObjects: true }) as Record<string, string>));
</script>

<section class="panel cut box">
  <ul>
    {#each rules as rule (rule)}
      <li><span class="check"><Check size={14} /></span>{rule}</li>
    {/each}
  </ul>
  <p class="muted note">{t('settings:securityNote')}</p>
</section>

<style>
  .box {
    padding: var(--space-4);
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: var(--space-3);
  }
  li {
    display: flex;
    gap: var(--space-3);
  }
  .check {
    display: flex;
    padding-top: var(--space-1);
    color: var(--color-success);
  }
  .note {
    margin: var(--space-5) 0 0;
    padding-top: var(--space-4);
    border-top: var(--border-hairline) solid var(--color-line);
  }
</style>
