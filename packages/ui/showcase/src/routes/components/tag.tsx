
import { createFileRoute } from '@tanstack/solid-router';
import { Tag } from '../../../../src';
import ComponentDemo from '../../components/ComponentDemo';

export const Route = createFileRoute('/components/tag')({
  component: TagPage,
});

function TagPage() {
  return (
    <div class="max-w-4xl">
      <div class="mb-8">
        <h1 class="text-4xl font-bold mb-4" style={`color: rgb(var(--lightSlate-50))`}>Tag</h1>
        <p class="text-xl" style={`color: rgb(var(--lightSlate-300))`}>
          Compact label component for categorizing, filtering, or displaying metadata.
        </p>
      </div>

      <ComponentDemo
        title="Basic Tags"
        description="Simple tags with different content"
      >
        <div class="flex flex-wrap gap-2">
          <Tag>JavaScript</Tag>
          <Tag>React</Tag>
          <Tag>SolidJS</Tag>
          <Tag>TypeScript</Tag>
          <Tag>Vite</Tag>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Tag Variants"
        description="Different tag styles and colors"
      >
        <div class="flex flex-wrap gap-2">
          <Tag variant="primary">Primary</Tag>
          <Tag variant="secondary">Secondary</Tag>
          <Tag variant="success">Success</Tag>
          <Tag variant="warning">Warning</Tag>
          <Tag variant="error">Error</Tag>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Removable Tags"
        description="Tags with close button for removal"
      >
        <div class="flex flex-wrap gap-2">
          <Tag removable onRemove={() => alert('Tag removed!')}>Click to remove</Tag>
          <Tag removable>Another removable tag</Tag>
          <Tag removable variant="primary">Primary removable</Tag>
        </div>
      </ComponentDemo>
    </div>
  );
}
