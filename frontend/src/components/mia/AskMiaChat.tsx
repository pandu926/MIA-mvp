'use client';

import Link from 'next/link';
import { useEffect, useMemo, useState } from 'react';
import {
  FaArrowLeft,
  FaBolt,
  FaBrain,
  FaChartLine,
  FaCircleNotch,
  FaComments,
  FaLayerGroup,
  FaWandMagicSparkles,
} from 'react-icons/fa6';
import { api } from '@/lib/api';
import { ASK_MIA_LOADING_STEPS, ASK_MIA_PRESET_QUESTIONS, shortenAddress } from '@/lib/ask-mia';
import type { AskMiaResponse, AskMiaTraceStepResponse, InvestigationRunDetailResponse } from '@/lib/types';

interface AskMiaChatProps {
  tokenAddress: string;
  tokenLabel: string;
  runId?: string;
}

type ChatMessage =
  | {
      id: string;
      role: 'user';
      question: string;
    }
  | {
      id: string;
      role: 'assistant';
      response: AskMiaResponse;
    };

function chatChipStyle(active: boolean) {
  return active
    ? {
        background: 'rgba(111,141,255,0.14)',
        color: 'var(--primary)',
        borderColor: 'rgba(111,141,255,0.24)',
      }
    : {
        background: 'rgba(255,255,255,0.03)',
        color: 'var(--on-surface)',
        borderColor: 'rgba(148,160,194,0.14)',
      };
}

export function AskMiaChat({ tokenAddress, tokenLabel, runId }: AskMiaChatProps) {
  const [question, setQuestion] = useState('');
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [loadingStepIndex, setLoadingStepIndex] = useState(0);
  const [attachedRunDetail, setAttachedRunDetail] = useState<InvestigationRunDetailResponse | null>(null);

  const canSubmit = question.trim().length > 0 && question.trim().length <= 400 && !loading;
  const latestAssistantResponse = useMemo(
    () =>
      [...messages]
        .reverse()
        .find((message): message is Extract<ChatMessage, { role: 'assistant' }> => message.role === 'assistant')
        ?.response ?? null,
    [messages]
  );
  const helperText = useMemo(() => {
    const count = question.trim().length;
    if (count === 0) return 'Ask a direct question about this launch. MIA will show which internal reads it uses before it answers.';
    return `${count}/400 characters`;
  }, [question]);
  const visibleLoadingSteps = ASK_MIA_LOADING_STEPS.slice(0, Math.max(1, loadingStepIndex + 1));
  const attachedRunContext = latestAssistantResponse?.run_context ?? null;
  const suggestedQuestions = useMemo(() => {
    if (!attachedRunDetail) {
      return ASK_MIA_PRESET_QUESTIONS;
    }

    const run = attachedRunDetail.run;
    const dynamic = [
      `What changed in run ${shortenAddress(run.run_id, 8, 6)}?`,
      `Why is this run ${run.status}?`,
      `What is MIA still monitoring for this run?`,
    ];

    return [...dynamic, ...ASK_MIA_PRESET_QUESTIONS].slice(0, 5);
  }, [attachedRunDetail]);

  useEffect(() => {
    if (!runId) {
      setAttachedRunDetail(null);
      return;
    }

    let active = true;
    api.investigations
      .getRunDetail(runId)
      .then((detail) => {
        if (!active) return;
        setAttachedRunDetail(detail);
      })
      .catch(() => {
        if (!active) return;
        setAttachedRunDetail(null);
      });

    return () => {
      active = false;
    };
  }, [runId]);

  const submit = async (nextQuestion?: string) => {
    const finalQuestion = (nextQuestion ?? question).trim();
    if (!finalQuestion) return;

    const requestId = `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
    setMessages((current) => [...current, { id: `${requestId}-user`, role: 'user', question: finalQuestion }]);
    setQuestion('');
    setError(null);
    setLoading(true);
    setLoadingStepIndex(0);

    const interval = window.setInterval(() => {
      setLoadingStepIndex((current) => Math.min(current + 1, ASK_MIA_LOADING_STEPS.length - 1));
    }, 850);

    try {
      const response = await api.tokens.askMia(tokenAddress, {
        question: finalQuestion,
        ...(runId ? { run_id: runId } : {}),
      });
      setMessages((current) => [...current, { id: `${requestId}-assistant`, role: 'assistant', response }]);
    } catch (err) {
      setError(
        err instanceof Error
          ? err.message
          : 'MIA could not answer right now. Try again in a few seconds.'
      );
    } finally {
      window.clearInterval(interval);
      setLoading(false);
    }
  };

  return (
    <div className="space-y-6">
      <section
        className="relative overflow-hidden rounded-[1.8rem] border p-5 shadow-[0_30px_80px_rgba(8,12,24,0.34)] md:p-7"
        style={{ background: 'linear-gradient(135deg, rgba(111,141,255,0.22), rgba(13,20,32,0.98) 55%, rgba(12,28,24,0.98))', borderColor: 'rgba(148,160,194,0.16)' }}
      >
        <div
          className="pointer-events-none absolute -right-12 top-0 h-44 w-44 rounded-full blur-3xl"
          style={{ background: 'rgba(111,141,255,0.22)' }}
        />
        <div
          className="pointer-events-none absolute bottom-0 left-12 h-32 w-32 rounded-full blur-3xl"
          style={{ background: 'rgba(0,255,163,0.1)' }}
        />

        <div className="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
          <div className="relative">
            <div className="flex items-center gap-2">
              <span style={{ color: 'var(--primary)' }}>
                <FaBrain size={14} />
              </span>
              <p className="text-[10px] font-bold uppercase tracking-[0.22em]" style={{ color: 'var(--outline)' }}>
                Ask MIA chat
              </p>
            </div>
            <h1 className="mt-2 font-headline text-2xl font-bold tracking-tight md:text-3xl">
              Ask direct questions about {tokenLabel}.
            </h1>
            <p className="mt-2 max-w-3xl text-sm leading-7" style={{ color: 'var(--on-surface-variant)' }}>
              This workspace is for question-driven analysis. MIA reads the launch from internal tools, then answers in plain language with evidence and the next move.
            </p>

            <div className="mt-4 flex flex-wrap gap-2">
              <HeaderPill icon={<FaLayerGroup size={12} />} label="Tool-routed answers" />
              <HeaderPill icon={<FaComments size={12} />} label="Chat-native workflow" />
              <HeaderPill icon={<FaChartLine size={12} />} label="Grounded in MIA signals" />
            </div>
          </div>
          <div className="rounded-2xl border px-4 py-4 text-sm shadow-[inset_0_1px_0_rgba(255,255,255,0.05)]" style={{ background: 'rgba(111,141,255,0.14)', borderColor: 'rgba(111,141,255,0.22)' }}>
            <p className="text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--outline)' }}>
              Active token
            </p>
            <p className="mt-1 font-semibold">{tokenLabel}</p>
            <p className="mt-1 text-xs" style={{ color: 'var(--on-surface-variant)' }}>
              {shortenAddress(tokenAddress, 10, 6)}
            </p>
            {attachedRunDetail && (
              <div className="mt-3 space-y-1" data-testid="ask-mia-run-context">
                <p className="text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--outline)' }}>
                  Attached run
                </p>
                <p className="text-xs font-semibold">
                  {shortenAddress(attachedRunDetail.run.run_id, 8, 6)} • {attachedRunDetail.run.status} • {attachedRunDetail.run.current_stage}
                </p>
              </div>
            )}
            <p className="mt-3 text-xs leading-6" style={{ color: 'var(--on-surface-variant)' }}>
              Use this workspace when the main report gives you a verdict, but you still need a sharper explanation or next move.
            </p>
          </div>
        </div>
      </section>

      <div className="grid gap-6 xl:grid-cols-[minmax(0,1.4fr)_22rem]">
        <section
          className="rounded-[1.6rem] border p-4 shadow-[0_24px_60px_rgba(8,12,24,0.24)] md:p-6"
          style={{ background: 'linear-gradient(180deg, rgba(255,255,255,0.025), rgba(255,255,255,0.01)), var(--surface-container-low)', borderColor: 'rgba(148,160,194,0.16)' }}
        >
          <div className="flex items-center justify-between gap-3">
            <div>
              <p className="text-[10px] font-bold uppercase tracking-[0.22em]" style={{ color: 'var(--outline)' }}>
                Chat workspace
              </p>
              <p className="mt-1 text-sm" style={{ color: 'var(--on-surface-variant)' }}>
                Ask MIA something specific, then inspect the answer and the tool activity behind it.
              </p>
            </div>
            <Link
              href={`/mia?q=${encodeURIComponent(tokenAddress)}`}
              className="inline-flex items-center gap-2 rounded-xl border px-4 py-3 text-xs font-bold uppercase tracking-[0.16em]"
              style={{ background: 'rgba(255,255,255,0.03)', color: 'var(--primary)', borderColor: 'rgba(148,160,194,0.14)' }}
            >
              <FaArrowLeft size={12} />
              Back to report
            </Link>
          </div>

          <div className="mt-5 rounded-[1.4rem] border p-4 shadow-[inset_0_1px_0_rgba(255,255,255,0.03)] md:p-5" style={{ background: 'rgba(10,14,22,0.62)', borderColor: 'rgba(148,160,194,0.12)' }}>
            <div className="flex flex-wrap gap-2">
              {suggestedQuestions.map((preset) => (
                <button
                  key={preset}
                  type="button"
                  disabled={loading}
                  onClick={() => void submit(preset)}
                  className="rounded-full border px-3 py-2 text-xs font-semibold transition-colors disabled:opacity-60"
                  style={chatChipStyle(question.trim() === preset)}
                >
                  {preset}
                </button>
              ))}
            </div>

            <div className="mt-5 flex flex-col gap-3 lg:flex-row">
              <div
                className="flex flex-1 items-center rounded-2xl border px-4 py-4"
                style={{ background: 'rgba(12,16,24,0.66)', borderColor: 'rgba(148,160,194,0.18)' }}
              >
                <textarea
                  data-testid="ask-mia-chat-input"
                  value={question}
                  onChange={(event) => setQuestion(event.target.value)}
                  placeholder="Ask something specific about this launch..."
                  rows={3}
                  className="w-full resize-none border-none bg-transparent text-sm leading-7 focus:ring-0"
                />
              </div>
              <button
                type="button"
                onClick={() => void submit()}
                disabled={!canSubmit}
                data-testid="ask-mia-chat-submit"
                className="rounded-2xl px-5 py-4 text-sm font-bold uppercase tracking-[0.18em] shadow-[0_18px_36px_rgba(79,110,255,0.24)] disabled:opacity-50"
                style={{ background: 'linear-gradient(135deg, rgba(111,141,255,1), rgba(124,147,255,0.92))', color: 'var(--on-primary-container)' }}
              >
                {loading ? 'Thinking...' : 'Send to MIA'}
              </button>
            </div>

            <p className="mt-3 text-xs" style={{ color: 'var(--on-surface-variant)' }}>
              {helperText}
            </p>
          </div>

          <div className="mt-5 space-y-4">
          {messages.length === 0 && !loading && (
            <div className="rounded-[1.4rem] border p-5" style={{ background: 'rgba(255,255,255,0.03)', borderColor: 'rgba(148,160,194,0.14)' }}>
              <div className="flex items-start gap-3">
                <div className="mt-1 rounded-full p-2" style={{ background: 'rgba(111,141,255,0.12)', color: 'var(--primary)' }}>
                  <FaComments size={13} />
                </div>
                <div>
                  <p className="text-sm font-semibold">No messages yet.</p>
                  <p className="mt-2 text-sm leading-7" style={{ color: 'var(--on-surface-variant)' }}>
                    Start with a direct question like <span style={{ color: 'var(--primary)' }}>“Why is this risky?”</span> or
                    <span style={{ color: 'var(--primary)' }}> “Compare this with the deployer&apos;s previous launches.”</span>
                  </p>
                </div>
              </div>
            </div>
          )}

          {messages.map((message) =>
            message.role === 'user' ? (
              <div key={message.id} className="flex justify-end">
                <div className="max-w-3xl rounded-[1.4rem] border px-5 py-4 shadow-[0_18px_36px_rgba(79,110,255,0.2)]" style={{ background: 'linear-gradient(135deg, rgba(111,141,255,1), rgba(124,147,255,0.92))', color: 'var(--on-primary-container)', borderColor: 'rgba(111,141,255,0.28)' }}>
                  <p className="text-[10px] font-bold uppercase tracking-[0.2em]" style={{ opacity: 0.78 }}>
                    You asked
                  </p>
                  <p className="mt-2 text-sm font-semibold">{message.question}</p>
                </div>
              </div>
            ) : (
              <AssistantMessage key={message.id} response={message.response} />
            )
          )}

          {loading && (
            <div className="max-w-4xl rounded-[1.4rem] border p-5" style={{ background: 'rgba(255,255,255,0.03)', borderColor: 'rgba(148,160,194,0.14)' }}>
              <div className="flex items-center gap-2">
                <FaCircleNotch className="animate-spin" size={14} style={{ color: 'var(--primary)' }} />
                <p className="text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--outline)' }}>
                  MIA is thinking
                </p>
              </div>
              <p className="mt-2 text-sm leading-7" style={{ color: 'var(--on-surface-variant)' }}>
                MIA is checking internal tools before it writes the answer.
              </p>

              <div className="mt-4 space-y-3" data-testid="ask-mia-thinking-trace">
                {visibleLoadingSteps.map((step, index) => (
                  <div key={`${step.tool}-${index}`} className="flex items-start gap-3">
                    <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-full border text-xs font-bold" style={{ background: 'rgba(111,141,255,0.12)', borderColor: 'rgba(111,141,255,0.22)', color: 'var(--primary)' }}>
                      {index + 1}
                    </div>
                    <div
                      className="flex-1 rounded-xl border px-4 py-3"
                      style={{ background: 'rgba(111,141,255,0.08)', borderColor: 'rgba(111,141,255,0.16)' }}
                    >
                      <div className="flex items-center gap-2">
                        <FaWandMagicSparkles size={12} style={{ color: 'var(--primary)' }} />
                        <p className="text-sm font-semibold">{step.title}</p>
                      </div>
                      <p className="mt-1 text-xs leading-6" style={{ color: 'var(--on-surface-variant)' }}>
                        {step.detail}
                      </p>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {error && (
            <div
              className="rounded-xl border px-4 py-3 text-sm"
              style={{ background: 'rgba(255,107,107,0.08)', borderColor: 'rgba(255,107,107,0.2)', color: 'var(--danger)' }}
            >
              MIA could not answer right now. Try again in a few seconds.
            </div>
          )}
        </div>
        </section>

        <aside className="space-y-4">
          <SidebarCard
            title="How MIA will read this"
            icon={<FaLayerGroup size={13} />}
            body="MIA routes the question through internal market, risk, builder, wallet, ML, and narrative tools only when they are useful for the answer."
          >
            <div className="mt-4 grid gap-2">
              <SidebarTag label="Risk and concentration" />
              <SidebarTag label="Wallet map and owner signals" />
              <SidebarTag label="Builder memory and repeat launches" />
              <SidebarTag label="Market structure and momentum" />
            </div>
          </SidebarCard>

          <SidebarCard
            title="Latest chat status"
            icon={<FaChartLine size={13} />}
            body={
              latestAssistantResponse
                ? 'The latest answer is grounded and ready to inspect.'
                : 'No answer yet. Ask a question to open the latest tool path.'
            }
          >
            <div className="mt-4 grid gap-3">
              <SidebarMetric
                label="Mode"
                value={latestAssistantResponse ? latestAssistantResponse.mode.replace('_', ' ') : 'Idle'}
              />
              <SidebarMetric
                label="Provider"
                value={latestAssistantResponse?.provider ?? 'Waiting'}
              />
              <SidebarMetric
                label="Internal tools"
                value={latestAssistantResponse ? String(latestAssistantResponse.tool_trace.length) : '0'}
              />
              <SidebarMetric
                label="Attached run"
                value={
                  attachedRunContext
                    ? `${attachedRunContext.status} • ${attachedRunContext.current_stage}`
                    : attachedRunDetail
                      ? `${attachedRunDetail.run.status} • ${attachedRunDetail.run.current_stage}`
                      : 'Detached'
                }
              />
            </div>
          </SidebarCard>

          <SidebarCard
            title="Best questions"
            icon={<FaWandMagicSparkles size={13} />}
            body={
              attachedRunDetail
                ? 'These prompts are derived from the attached run so chat can explain continuity, state changes, and monitoring gaps.'
                : 'Use this lane when you need interpretation, not just raw evidence.'
            }
          >
            <ul className="mt-4 space-y-3 text-sm" style={{ color: 'var(--on-surface-variant)' }}>
              {suggestedQuestions.slice(0, 3).map((item) => (
                <li key={item}>{item}</li>
              ))}
            </ul>
          </SidebarCard>
        </aside>
      </div>
    </div>
  );
}

function AssistantMessage({ response }: { response: AskMiaResponse }) {
  return (
    <div className="flex justify-start">
      <div
        data-testid="ask-mia-chat-response"
        className="max-w-4xl rounded-[1.4rem] border p-5 shadow-[0_20px_50px_rgba(8,12,24,0.22)]"
        style={{ background: 'rgba(255,255,255,0.04)', borderColor: 'rgba(148,160,194,0.14)' }}
      >
        <div className="flex flex-wrap items-center gap-2 text-[11px] uppercase tracking-widest">
          <span className="rounded-full px-3 py-1" style={{ background: 'rgba(105,137,255,0.12)', color: 'var(--primary)' }}>
            {response.provider}
          </span>
          <span className="rounded-full px-3 py-1" style={{ background: 'rgba(255,255,255,0.06)', color: 'var(--on-surface)' }}>
            {response.mode.replace('_', ' ')}
          </span>
          <span className="rounded-full px-3 py-1" style={{ background: 'rgba(0,255,163,0.08)', color: 'var(--secondary-container)' }}>
            {response.grounded_layers.length} grounded layers
          </span>
          {response.run_context && (
            <span
              data-testid="ask-mia-chat-run-aware"
              className="rounded-full px-3 py-1"
              style={{ background: 'rgba(255,186,73,0.12)', color: 'var(--warning)' }}
            >
              run-aware
            </span>
          )}
          {response.tool_trace.length > 0 && (
            <span className="rounded-full px-3 py-1" style={{ background: 'rgba(105,137,255,0.08)', color: 'var(--primary)' }}>
              {response.tool_trace.length} internal tools
            </span>
          )}
        </div>

        {response.run_context && (
          <div
            className="mt-4 rounded-xl border p-4"
            style={{ background: 'rgba(255,186,73,0.08)', borderColor: 'rgba(255,186,73,0.18)' }}
          >
            <p className="text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--outline)' }}>
              Attached run context
            </p>
            <p className="mt-2 text-sm font-semibold">
              {shortenAddress(response.run_context.run_id, 8, 6)} • {response.run_context.status} • {response.run_context.current_stage}
            </p>
            <p className="mt-2 text-sm leading-7">{response.run_context.continuity_note}</p>
          </div>
        )}

        <div className="mt-4 rounded-xl border p-4" style={{ background: 'rgba(105,137,255,0.08)', borderColor: 'rgba(105,137,255,0.16)' }}>
          <p className="text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--outline)' }}>
            Short answer
          </p>
          <p className="mt-2 text-base font-semibold" data-testid="ask-mia-chat-short-answer">
            {response.answer.short_answer}
          </p>
        </div>

        <div className="mt-5 grid gap-4 lg:grid-cols-2">
          <div className="rounded-xl border p-4" style={{ background: 'var(--surface-container-lowest)', borderColor: 'rgba(141,144,161,0.12)' }}>
            <p className="text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--outline)' }}>
              Why MIA says this
            </p>
            <p className="mt-2 text-sm leading-7">{response.answer.why}</p>
          </div>
          <div className="rounded-xl border p-4" style={{ background: 'var(--surface-container-lowest)', borderColor: 'rgba(141,144,161,0.12)' }}>
            <p className="text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--outline)' }}>
              What to do next
            </p>
            <p className="mt-2 text-sm leading-7">{response.answer.next_move}</p>
          </div>
        </div>

        <div className="mt-5 rounded-xl border p-4" style={{ background: 'var(--surface-container-lowest)', borderColor: 'rgba(141,144,161,0.12)' }}>
          <div className="flex items-center gap-2">
            <span style={{ color: 'var(--primary)' }}>
              <FaBolt size={12} />
            </span>
            <p className="text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--outline)' }}>
              Evidence used
            </p>
          </div>
          <div className="mt-3 space-y-2">
            {response.answer.evidence.map((item) => (
              <div
                key={item}
                className="rounded-lg px-3 py-2 text-sm"
                style={{ background: 'rgba(0,255,163,0.08)', color: 'var(--secondary-container)' }}
              >
                {item}
              </div>
            ))}
          </div>
        </div>

        {response.analysis_trace.length > 0 && (
          <div className="mt-5 rounded-xl border p-4" style={{ background: 'var(--surface-container-lowest)', borderColor: 'rgba(141,144,161,0.12)' }}>
            <p className="text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--outline)' }}>
              Tool activity
            </p>
            <div className="mt-3 space-y-3" data-testid="ask-mia-chat-tool-activity">
              {response.analysis_trace.map((step: AskMiaTraceStepResponse, index) => (
                <div key={`${response.generated_at}-${step.tool}`} className="flex items-start gap-3">
                  <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-full border text-xs font-bold" style={{ background: 'rgba(111,141,255,0.12)', borderColor: 'rgba(111,141,255,0.22)', color: 'var(--primary)' }}>
                    {index + 1}
                  </div>
                  <div
                    className="flex-1 rounded-xl border px-4 py-3"
                    style={{ background: 'rgba(111,141,255,0.08)', borderColor: 'rgba(111,141,255,0.16)' }}
                  >
                    <div className="flex items-center gap-2">
                      <FaWandMagicSparkles size={12} style={{ color: 'var(--primary)' }} />
                      <p className="text-sm font-semibold">{step.title}</p>
                    </div>
                    <p className="mt-1 text-xs leading-6" style={{ color: 'var(--on-surface-variant)' }}>
                      {step.detail}
                    </p>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

function HeaderPill({ icon, label }: { icon: React.ReactNode; label: string }) {
  return (
    <span
      className="inline-flex items-center gap-2 rounded-full border px-3 py-2 text-[11px] font-semibold"
      style={{ background: 'rgba(255,255,255,0.035)', borderColor: 'rgba(148,160,194,0.14)', color: 'var(--on-surface)' }}
    >
      <span style={{ color: 'var(--primary)' }}>{icon}</span>
      {label}
    </span>
  );
}

function SidebarCard({
  title,
  icon,
  body,
  children,
}: {
  title: string;
  icon: React.ReactNode;
  body: string;
  children?: React.ReactNode;
}) {
  return (
    <section
      className="rounded-[1.4rem] border p-5 shadow-[0_24px_60px_rgba(8,12,24,0.22)]"
      style={{ background: 'linear-gradient(180deg, rgba(255,255,255,0.025), rgba(255,255,255,0.01)), var(--surface-container-low)', borderColor: 'rgba(148,160,194,0.16)' }}
    >
      <div className="flex items-center gap-2">
        <span style={{ color: 'var(--primary)' }}>{icon}</span>
        <p className="text-[10px] font-bold uppercase tracking-[0.22em]" style={{ color: 'var(--outline)' }}>
          {title}
        </p>
      </div>
      <p className="mt-3 text-sm leading-7" style={{ color: 'var(--on-surface-variant)' }}>
        {body}
      </p>
      {children}
    </section>
  );
}

function SidebarTag({ label }: { label: string }) {
  return (
    <div
      className="rounded-xl border px-3 py-2 text-xs font-semibold"
      style={{ background: 'rgba(111,141,255,0.08)', borderColor: 'rgba(111,141,255,0.14)', color: 'var(--primary)' }}
    >
      {label}
    </div>
  );
}

function SidebarMetric({ label, value }: { label: string; value: string }) {
  return (
    <div
      className="rounded-xl border px-4 py-3"
      style={{ background: 'rgba(255,255,255,0.025)', borderColor: 'rgba(148,160,194,0.12)' }}
    >
      <p className="text-[10px] font-bold uppercase tracking-[0.2em]" style={{ color: 'var(--outline)' }}>
        {label}
      </p>
      <p className="mt-2 text-sm font-semibold">{value}</p>
    </div>
  );
}
