
import { createFileRoute } from '@tanstack/solid-router';
import { Collapsable } from '../../../../src';
import ComponentDemo from '../../components/ComponentDemo';
import { createSignal } from 'solid-js';

export const Route = createFileRoute('/components/collapsable')({
  component: CollapsablePage,
});

function CollapsablePage() {
  const [isOpen1, setIsOpen1] = createSignal(false);
  const [isOpen2, setIsOpen2] = createSignal(true);

  return (
    <div class="max-w-4xl">
      <div class="mb-8">
        <h1 class="text-4xl font-bold mb-4" style={`color: rgb(var(--lightSlate-50))`}>Collapsable</h1>
        <p class="text-xl" style={`color: rgb(var(--lightSlate-300))`}>
          Collapsible content container that can be toggled to show or hide content.
        </p>
      </div>

      <ComponentDemo
        title="Basic Collapsable"
        description="Simple collapsible content with toggle functionality"
      >
        <Collapsable 
          isOpen={isOpen1()} 
          onToggle={setIsOpen1}
          title="Click to expand"
        >
          <div style={`color: rgb(var(--lightSlate-100))`}>
            <p class="mb-2">This content can be collapsed and expanded.</p>
            <p>It's useful for organizing content in a compact way while still allowing access to detailed information when needed.</p>
          </div>
        </Collapsable>
      </ComponentDemo>

      <ComponentDemo
        title="Initially Open"
        description="Collapsable that starts in the open state"
      >
        <Collapsable 
          isOpen={isOpen2()} 
          onToggle={setIsOpen2}
          title="Already expanded"
        >
          <div style={`color: rgb(var(--lightSlate-100))`}>
            <p class="mb-2">This collapsible starts in the open state.</p>
            <p>You can click the title to collapse it.</p>
          </div>
        </Collapsable>
      </ComponentDemo>
    </div>
  );
}
