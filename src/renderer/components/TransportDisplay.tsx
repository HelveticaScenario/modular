import type { TransportSnapshot } from '../../shared/ipcTypes';

interface TransportDisplayProps {
    transport: TransportSnapshot | null;
    onToggleLink?: (enabled: boolean) => void;
}

export function TransportDisplay({
    transport,
    onToggleLink,
}: TransportDisplayProps) {
    if (!transport) {
        return (
            <div className="transport-display">
                <span className="transport-position transport-dim">---.--</span>
            </div>
        );
    }

    const {
        bpm,
        timeSigNumerator,
        timeSigDenominator,
        bar,
        beatInBar,
        hasQueuedUpdate,
    } = transport;

    // Display bar as 1-indexed
    const displayBar = bar + 1;
    // Beat is 0-indexed in the data, display as 1-indexed
    const displayBeat = beatInBar + 1;

    // Beat indicator pips
    const pips: boolean[] = [];
    for (let i = 0; i < timeSigNumerator; i++) {
        pips.push(i === beatInBar);
    }

    return (
        <div className="transport-display">
            {/* Tempo */}
            <span className="transport-tempo" title="Tempo (BPM)">
                {bpm.toFixed(0)}
            </span>

            {/* Time signature */}
            <span
                className="transport-timesig transport-dim"
                title="Time signature"
            >
                {timeSigNumerator}/{timeSigDenominator}
            </span>

            {/* Bar.Beat position */}
            <span className="transport-position" title="Bar.Beat">
                {displayBar}.{displayBeat}
            </span>

            {/* Beat pips */}
            <span className="transport-pips" title="Beat position">
                {pips.map((active, i) => (
                    <span
                        key={i}
                        className={`transport-pip${active ? ' active' : ''}`}
                    />
                ))}
            </span>

            {/* Link toggle */}
            <button
                className={`transport-link${transport.linkEnabled ? ' active' : ''}`}
                onClick={() => onToggleLink?.(!transport.linkEnabled)}
                title={
                    transport.linkEnabled
                        ? `Link active (${transport.linkPeers} peer${transport.linkPeers !== 1 ? 's' : ''})`
                        : 'Enable Ableton Link'
                }
            >
                Link
                {transport.linkEnabled && transport.linkPeers > 0 && (
                    <span className="transport-link-peers">
                        {transport.linkPeers}
                    </span>
                )}
            </button>

            {/* Queued update indicator */}
            {hasQueuedUpdate && (
                <span className="transport-queued" title="Update queued">
                    Q
                </span>
            )}
        </div>
    );
}
