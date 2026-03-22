import { useState } from 'react'

// ── Template data by practice area ──────────────────────────────────────────

const TEMPLATES: Record<string, string[]> = {
  General: [
    'Summarize this document',
    'What are the key dates?',
    'Who are the parties involved?',
  ],
  'Criminal Law': [
    'What charges are described?',
    'What is the statute of limitations?',
    'What evidence is referenced?',
  ],
  'Family / Domestic': [
    'What are the custody terms?',
    'What is the support amount?',
    'What are the visitation rights?',
  ],
  'Corporate / Contract': [
    'What are the termination clauses?',
    'What are the payment terms?',
    'What warranties are provided?',
  ],
  Immigration: [
    'What visa type is referenced?',
    'What are the filing deadlines?',
    'What evidence is required?',
  ],
  'Personal Injury': [
    'What injuries are described?',
    'What damages are claimed?',
    'What is the statute of limitations?',
  ],
  'Real Estate / Property': [
    'What is the monthly rent?',
    'When does the lease end?',
    'What are the maintenance obligations?',
  ],
  'Employment / Labor': [
    'What is the non-compete scope?',
    'What are the benefits?',
    'What are the termination conditions?',
  ],
  'Regulatory / Compliance': [
    'What regulations are cited?',
    'What are the compliance deadlines?',
    'What penalties apply?',
  ],
}

interface Props {
  practiceArea: string | null
  onSelect: (query: string) => void
}

export default function QueryTemplates({ practiceArea, onSelect }: Props): JSX.Element {
  const [hoveredIdx, setHoveredIdx] = useState<number | null>(null)
  const templates = TEMPLATES[practiceArea ?? 'General'] ?? TEMPLATES.General

  return (
    <div className="flex flex-wrap gap-2 justify-center" style={{ maxWidth: 520 }}>
      {templates.map((t, i) => (
        <button
          key={t}
          onClick={() => onSelect(t)}
          onMouseEnter={() => setHoveredIdx(i)}
          onMouseLeave={() => setHoveredIdx(null)}
          className="rounded-xl px-3.5 py-2 text-[11.5px] font-medium transition-all"
          style={{
            background: hoveredIdx === i ? 'rgba(201,168,76,0.1)' : 'rgb(var(--ov) / 0.03)',
            border: `1px solid ${hoveredIdx === i ? 'rgba(201,168,76,0.3)' : 'rgb(var(--ov) / 0.08)'}`,
            color: hoveredIdx === i ? '#c9a84c' : 'rgb(var(--ov) / 0.45)',
            cursor: 'pointer',
          }}
        >
          {t}
        </button>
      ))}
    </div>
  )
}
