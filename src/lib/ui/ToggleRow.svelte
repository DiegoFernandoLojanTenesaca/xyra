<script lang="ts">
  let { title, description = '', checked, onchange }: { title: string; description?: string; checked: boolean; onchange: () => void } = $props();
</script>

<label>
  <span
    ><b>{title}</b>{#if description}<small class="muted">{description}</small>{/if}</span
  >
  <input type="checkbox" class="switch" {checked} {onchange} />
</label>

<style>
  label {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
    padding: var(--space-3) 0;
    cursor: pointer;
  }
  label + :global(label) {
    border-top: var(--border-hairline) solid var(--color-line);
  }
  span {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }
  b {
    font-weight: 600;
  }
  small {
    font-size: var(--text-sm);
  }
  .switch {
    appearance: none;
    position: relative;
    flex: none;
    width: var(--size-switch);
    height: var(--space-6);
    margin: 0;
    cursor: pointer;
    background: var(--color-line);
    transition: background 0.15s;
  }
  .switch::after {
    content: '';
    position: absolute;
    top: var(--space-1);
    left: var(--space-1);
    width: var(--space-4);
    height: var(--space-4);
    background: var(--color-textSubtle);
    transition: transform 0.15s;
  }
  .switch:checked {
    background: var(--color-accent);
  }
  .switch:checked::after {
    transform: translateX(var(--space-5));
    background: var(--color-white);
  }
</style>
