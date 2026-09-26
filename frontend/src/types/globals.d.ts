export { };

declare global {
    interface Window {
        DEVICE_IP?: string;
        DEVICES?: DashboardDevice[];
        TRACEPULSE_DISCOVERY_LIMITS?: {
            maxCidrs?: number | null;
            nodeLimit?: number;
        };
        TRACEPULSE_SPIKE_THRESHOLD?: number;
        IFACE_LABELS?: Record<string, string>;
        esc: (value: unknown) => string;
        t?: (key: string) => string;
        tf?: (key: string, params?: Record<string, string | number>) => string;
        registerTranslations?: (translations: { en?: Record<string, string>; ja?: Record<string, string> }) => void;
        isInterfaceSelected?: (ifIndex: unknown) => boolean;
        loadDeviceDetail?: () => void;
        renderDetail?: (device: DeviceDetail) => void;
        renderTrafficProtocols?: (data: FlowAnalytics | null) => void;
        refreshTrafficProtocols?: () => void;
        selectAllInterfaces?: (checked: boolean) => void;
        formatBps?: (value: unknown) => string;
        fmtNum?: (value: unknown) => string;
        fmtTime?: (iso: unknown) => string;
        formatTracePulseClock?: (value: Date) => string;
        formatTracePulseTime?: (value: Date) => string;
        parseTracePulseTime?: (iso: unknown) => Date;
        LAST_DEVICE_DETAIL?: DeviceDetail;
        onErrorBreakdownIfaceChange?: (value: string) => void;
        renderAlerts?: (alerts: AlertRow[]) => void;
        renderBwChart?: (ifSeries: InterfaceSeries[]) => void;
        renderCpuChart?: (metrics: SystemMetric[]) => void;
        renderErrChart?: (ifSeries: InterfaceSeries[]) => void;
        renderErrorBreakdownCard?: (ifaces: DeviceInterface[]) => void;
        renderHardwareStatus?: (sensors: HardwareSensor[]) => void;
        renderInterfaces?: (ifaces: DeviceInterface[]) => void;
        renderMemoryChart?: (metrics: SystemMetric[]) => void;
        renderSpikes?: (spikes: InterfaceSpike[]) => void;
        renderSysChart?: (metrics: SystemMetric[]) => void;
        tracePulseTimeZone?: () => string;
        sparkline?: (svgId: string, series: ChartSeries[], yLabel: string) => void;
        toggleInterfaceSelection?: (ifIndex: unknown, checked: boolean) => void;
        renderTable?: () => void;
        renderSummary?: (devices?: DashboardDevice[]) => void;
        refreshDashboard?: () => void;
        sortBy?: (column: string) => void;
        unregisterDevice?: (ip: string, name: string) => void;
        INIT: SettingsFormValues;
        populateForm?: (values: SettingsFormValues) => void;
        resetDefaults?: () => void;
        saveSettings?: () => void;
        updateBar?: () => void;
        saveOidOverrides?: () => void;
        startDiagnostics?: (event: SubmitEvent) => boolean;
        openActiveDiagnostic?: (target: string, kind: string, port?: number) => void;
        openActiveDiagnosticWithPortPrompt?: (target: string, kind: string, port?: number) => void;
        registerDiagnosticExtension?: (extension: {
            tabs: Array<{ kind: string; label: string; portPrompt?: boolean; initialPort?: number; promptTitle?: string }>;
            resultTitles: Record<string, string>;
            timeouts: Record<string, number>;
            counts: Record<string, number>;
            reportTitle?: string;
            formatValue?: (type: string, key: string, value: unknown) => string | undefined;
        }) => void;
        closeActiveDiagnostic?: () => void;
        copyActiveDiagnostic?: () => Promise<void>;
        DISCOVERY_EXISTING?: Set<string>;
        startTopologyDiscovery?: (seedIp: string, community: string) => Promise<void>;
        addManual?: () => Promise<void>;
        cancelScan?: () => void;
        hideManual?: () => void;
        registerSelected?: () => Promise<void>;
        runTopologyOnly?: () => Promise<void>;
        showManual?: () => void;
        startScan?: () => Promise<void>;
        toggleAll?: (master: HTMLInputElement) => void;
        updateHint?: () => void;
        updateRegisterBtn?: () => void;
        closeInspector?: () => void;
        exportTopology?: (format: string) => void;
        refreshTopologyTexts?: () => void;
        applyPageLanguage?: (lang?: string) => void;
        I18N?: { en?: Record<string, string>; ja?: Record<string, string> };
        currentLanguage?: () => string;
        setLanguage?: (lang: string) => void;
        currentTheme?: () => string;
        setTheme?: (theme: string) => void;
        applyTheme?: (theme?: string) => void;
        applyLanguage?: (lang?: string) => void;
        formatTracePulseTimestamp?: (value: unknown) => string;
        topoFit?: () => void;
        topoRelayout?: () => void;
        topoZoom?: (factor: number) => void;
    }

    interface ErrorBreakdown {
        fcs_errors?: number;
        fcs_errors_delta?: number;
        alignment_errors?: number;
        alignment_errors_delta?: number;
        frame_too_longs?: number;
        frame_too_longs_delta?: number;
        internal_mac_receive_errors?: number;
        internal_mac_receive_errors_delta?: number;
    }

    interface DashboardDevice {
        ip: string;
        name?: string;
        status?: string;
        community?: string;
        last_seen?: string;
        last_seen_at?: string;
        error_spike?: boolean;
    }

    interface SettingsFormValues {
        interval: number;
        community: string;
        timezone: string;
        error_rate: number;
        spike: number;
        warn: number;
        crit: number;
        days: number;
    }

    interface DeviceInterface {
        if_index: number;
        if_name?: string;
        link_status?: string;
        health_status?: string;
        metrics?: InterfaceMetrics;
        error_breakdown?: ErrorBreakdown;
        sampled_at?: string;
    }

    interface InterfaceMetrics {
        in_errors?: number;
        in_errors_delta?: number;
        out_errors?: number;
        out_errors_delta?: number;
        in_discards?: number;
        in_discards_delta?: number;
        out_discards?: number;
        out_discards_delta?: number;
        late_collisions?: number;
        late_collisions_delta?: number;
        bandwidth_utilization?: number;
    }

    interface DeviceDetail {
        name?: string;
        ip?: string;
        status?: string;
        error?: string;
        interfaces?: DeviceInterface[];
        if_series?: InterfaceSeries[];
        metrics?: SystemMetric[];
        hardware_sensors?: HardwareSensor[];
        spikes?: InterfaceSpike[];
        alerts?: AlertRow[];
    }

    interface HardwareSensor {
        sensor_type?: string;
        value?: string | number | null;
        status?: string | number | null;
        status_text?: string | null;
        unit?: string | null;
        name?: string | null;
        source?: string | null;
        index?: string | number | null;
        is_alarm?: boolean;
    }

    interface InterfaceSpike {
        latest_sampled_at?: string;
        if_name?: string;
        if_index?: number;
        link_status?: string;
        in_errors_delta?: number;
        out_errors_delta?: number;
        in_discards_delta?: number;
        out_discards_delta?: number;
        total_delta?: number;
    }

    interface AlertRow {
        at?: string;
        interface?: string;
        type?: string;
        severity?: string;
        details?: string;
    }

    interface ChartPoint {
        t: string;
        v: number;
    }

    interface ChartSeries {
        label: string;
        color: string;
        points: ChartPoint[];
    }

    interface InterfaceSeriesPoint {
        t: string;
        bw?: number;
        in_err?: number;
        in_dis?: number;
    }

    interface InterfaceSeries {
        if_index: number;
        points: InterfaceSeriesPoint[];
    }

    interface SystemMetric {
        t: string;
        cpu?: number | null;
        memory?: number | null;
        memory_bytes?: number | null;
    }

    interface FlowAnalytics {
        protocols?: ProtocolShare[];
        top_talkers?: TopTalker[];
        summary?: { total_bps?: number; total_pps?: number; active_flows?: number; top_protocol?: string };
        applications?: { app_name: string; bytes?: number; percentage?: number; bps?: number }[];
        top_sources?: FlowEndpoint[];
        top_destinations?: FlowEndpoint[];
        timeseries?: FlowTimeseries[];
    }

    interface FlowEndpoint { ip: string; bps?: number; percentage?: number; }
    interface FlowTimeseries { timestamp: string; udp_bps?: number; tcp_bps?: number; icmp_bps?: number; }

    interface ProtocolShare {
        protocol: string;
        bytes?: number;
        bps?: number;
        percentage?: number;
        if_index?: number;
        pps?: number;
    }

    interface TopTalker {
        source_ip: string;
        destination_ip: string;
        source_port?: number;
        destination_port?: number;
        protocol: string;
        bytes?: number;
        bps?: number;
        if_index?: number;
        app_name?: string;
        packets?: number;
        pps?: number;
        tcp_flags?: string[];
        ingress_if_index?: number;
        egress_if_index?: number;
        ingress_if_name?: string;
        egress_if_name?: string;
    }
}
