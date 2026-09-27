//! Apache JMeter (.jmx) export helpers.

use crate::http::join_base_url;
use crate::models::{ExportCollection, ExportRequest, KeyValue};
use url::Url;

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn enabled_kv(rows: &[KeyValue]) -> Vec<&KeyValue> {
    rows.iter()
        .filter(|r| r.enabled && !r.key.trim().is_empty())
        .collect()
}

struct ParsedUrl {
    protocol: String,
    domain: String,
    port: String,
    path: String,
}

fn parse_request_url(raw: &str) -> ParsedUrl {
    let trimmed = raw.trim();
    if let Ok(u) = Url::parse(trimmed) {
        let protocol = u.scheme().to_string();
        let domain = u.host_str().unwrap_or("").to_string();
        let port = match u.port() {
            Some(p) => p.to_string(),
            None if protocol == "https" => "443".into(),
            None if protocol == "http" => "80".into(),
            None => String::new(),
        };
        let mut path = u.path().to_string();
        if path.is_empty() {
            path = "/".into();
        }
        if let Some(q) = u.query() {
            path = format!("{path}?{q}");
        }
        return ParsedUrl {
            protocol,
            domain,
            port,
            path,
        };
    }
    // Relative or placeholder URL — put everything in path for JMeter
    ParsedUrl {
        protocol: "https".into(),
        domain: String::new(),
        port: String::new(),
        path: if trimmed.is_empty() {
            "/".into()
        } else {
            trimmed.to_string()
        },
    }
}

fn resolve_url(base_url: &str, req: &ExportRequest) -> String {
    join_base_url(base_url, &req.url)
}

fn append_query_from_params(url: &str, params: &[KeyValue]) -> String {
    let enabled = enabled_kv(params);
    if enabled.is_empty() {
        return url.to_string();
    }
    if let Ok(mut parsed) = Url::parse(url) {
        {
            let mut qp = parsed.query_pairs_mut();
            for row in enabled {
                qp.append_pair(&row.key, &row.value);
            }
        }
        return parsed.to_string();
    }
    let mut out = url.to_string();
    let join = if out.contains('?') { '&' } else { '?' };
    let pairs: Vec<String> = enabled
        .iter()
        .map(|r| format!("{}={}", r.key, r.value))
        .collect();
    out.push(join);
    out.push_str(&pairs.join("&"));
    out
}

fn header_manager_xml(headers: &[KeyValue]) -> String {
    let enabled = enabled_kv(headers);
    if enabled.is_empty() {
        return String::new();
    }
    let mut props = String::new();
    for (i, h) in enabled.iter().enumerate() {
        props.push_str(&format!(
            r#"            <elementProp name="H{i}" elementType="Header">
              <stringProp name="Header.name">{name}</stringProp>
              <stringProp name="Header.value">{value}</stringProp>
            </elementProp>
"#,
            i = i,
            name = xml_escape(&h.key),
            value = xml_escape(&h.value),
        ));
    }
    format!(
        r#"          <HeaderManager guiclass="HeaderPanel" testclass="HeaderManager" testname="HTTP Headers" enabled="true">
            <collectionProp name="HeaderManager.headers">
{props}            </collectionProp>
          </HeaderManager>
          <hashTree/>
"#
    )
}

fn body_arguments_xml(req: &ExportRequest) -> String {
    let method = req.method.to_uppercase();
    if matches!(method.as_str(), "GET" | "HEAD") || req.body_type == "none" {
        return r#"          <boolProp name="HTTPSampler.postBodyRaw">false</boolProp>
          <elementProp name="HTTPsampler.Arguments" elementType="Arguments" guiclass="HTTPArgumentsPanel" testclass="Arguments" testname="User Defined Variables" enabled="true">
            <collectionProp name="Arguments.arguments"/>
          </elementProp>
"#
        .into();
    }

    match req.body_type.as_str() {
        "x-www-form-urlencoded" | "form-data" => {
            let rows: Vec<KeyValue> =
                serde_json::from_str(&req.body_content).unwrap_or_default();
            let enabled = enabled_kv(&rows);
            let mut args = String::new();
            for row in enabled {
                args.push_str(&format!(
                    r#"              <elementProp name="{name}" elementType="HTTPArgument">
                <boolProp name="HTTPArgument.always_encode">false</boolProp>
                <stringProp name="Argument.value">{value}</stringProp>
                <stringProp name="Argument.metadata">=</stringProp>
                <boolProp name="HTTPArgument.use_equals">true</boolProp>
                <stringProp name="Argument.name">{name}</stringProp>
              </elementProp>
"#,
                    name = xml_escape(&row.key),
                    value = xml_escape(&row.value),
                ));
            }
            format!(
                r#"          <boolProp name="HTTPSampler.postBodyRaw">false</boolProp>
          <elementProp name="HTTPsampler.Arguments" elementType="Arguments" guiclass="HTTPArgumentsPanel" testclass="Arguments" testname="User Defined Variables" enabled="true">
            <collectionProp name="Arguments.arguments">
{args}            </collectionProp>
          </elementProp>
"#
            )
        }
        _ => {
            // json / raw — postBodyRaw
            format!(
                r#"          <boolProp name="HTTPSampler.postBodyRaw">true</boolProp>
          <elementProp name="HTTPsampler.Arguments" elementType="Arguments">
            <collectionProp name="Arguments.arguments">
              <elementProp name="" elementType="HTTPArgument">
                <boolProp name="HTTPArgument.always_encode">false</boolProp>
                <stringProp name="Argument.value">{body}</stringProp>
                <stringProp name="Argument.metadata">=</stringProp>
              </elementProp>
            </collectionProp>
          </elementProp>
"#,
                body = xml_escape(&req.body_content),
            )
        }
    }
}

fn http_sampler_xml(req: &ExportRequest, base_url: &str) -> String {
    let url = append_query_from_params(&resolve_url(base_url, req), &req.params);
    let parsed = parse_request_url(&url);
    let name = if req.name.trim().is_empty() {
        "HTTP Request"
    } else {
        req.name.trim()
    };
    let method = if req.method.trim().is_empty() {
        "GET".into()
    } else {
        req.method.to_uppercase()
    };

    format!(
        r#"        <HTTPSamplerProxy guiclass="HttpTestSampleGui" testclass="HTTPSamplerProxy" testname="{name}" enabled="true">
{body}          <stringProp name="HTTPSampler.domain">{domain}</stringProp>
          <stringProp name="HTTPSampler.port">{port}</stringProp>
          <stringProp name="HTTPSampler.protocol">{protocol}</stringProp>
          <stringProp name="HTTPSampler.contentEncoding"></stringProp>
          <stringProp name="HTTPSampler.path">{path}</stringProp>
          <stringProp name="HTTPSampler.method">{method}</stringProp>
          <boolProp name="HTTPSampler.follow_redirects">true</boolProp>
          <boolProp name="HTTPSampler.auto_redirects">false</boolProp>
          <boolProp name="HTTPSampler.use_keepalive">true</boolProp>
          <boolProp name="HTTPSampler.DO_MULTIPART_POST">{multipart}</boolProp>
          <stringProp name="HTTPSampler.embedded_url_re"></stringProp>
          <stringProp name="HTTPSampler.connect_timeout"></stringProp>
          <stringProp name="HTTPSampler.response_timeout"></stringProp>
        </HTTPSamplerProxy>
        <hashTree>
{headers}        </hashTree>
"#,
        name = xml_escape(name),
        body = body_arguments_xml(req),
        domain = xml_escape(&parsed.domain),
        port = xml_escape(&parsed.port),
        protocol = xml_escape(&parsed.protocol),
        path = xml_escape(&parsed.path),
        method = xml_escape(&method),
        multipart = if req.body_type == "form-data" {
            "true"
        } else {
            "false"
        },
        headers = header_manager_xml(&req.headers),
    )
}

/// Build a JMeter 5.x compatible `.jmx` XML string from an ApiTest collection export.
pub fn collection_to_jmx(data: &ExportCollection, threads: u32, loops: i32, ramp_up: u32) -> String {
    let plan_name = if data.name.trim().is_empty() {
        "ApiTest Plan"
    } else {
        data.name.trim()
    };
    let threads = threads.clamp(1, 500);
    let loops = if loops <= 0 { 1 } else { loops };

    let mut samplers = String::new();
    if data.requests.is_empty() {
        samplers.push_str(
            r#"        <HTTPSamplerProxy guiclass="HttpTestSampleGui" testclass="HTTPSamplerProxy" testname="Empty" enabled="false">
          <stringProp name="HTTPSampler.path">/</stringProp>
          <stringProp name="HTTPSampler.method">GET</stringProp>
        </HTTPSamplerProxy>
        <hashTree/>
"#,
        );
    } else {
        for req in &data.requests {
            samplers.push_str(&http_sampler_xml(req, &data.base_url));
        }
    }

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<jmeterTestPlan version="1.2" properties="5.0" jmeter="5.6.3">
  <hashTree>
    <TestPlan guiclass="TestPlanGui" testclass="TestPlan" testname="{plan}" enabled="true">
      <stringProp name="TestPlan.comments">Exported from ApiTest</stringProp>
      <boolProp name="TestPlan.functional_mode">false</boolProp>
      <boolProp name="TestPlan.tearDown_on_shutdown">true</boolProp>
      <boolProp name="TestPlan.serialize_threadgroups">false</boolProp>
      <elementProp name="TestPlan.user_defined_variables" elementType="Arguments" guiclass="ArgumentsPanel" testclass="Arguments" testname="User Defined Variables" enabled="true">
        <collectionProp name="Arguments.arguments"/>
      </elementProp>
      <stringProp name="TestPlan.user_define_classpath"></stringProp>
    </TestPlan>
    <hashTree>
      <ThreadGroup guiclass="ThreadGroupGui" testclass="ThreadGroup" testname="Thread Group" enabled="true">
        <stringProp name="ThreadGroup.on_sample_error">continue</stringProp>
        <elementProp name="ThreadGroup.main_controller" elementType="LoopController" guiclass="LoopControlPanel" testclass="LoopController" testname="Loop Controller" enabled="true">
          <boolProp name="LoopController.continue_forever">false</boolProp>
          <stringProp name="LoopController.loops">{loops}</stringProp>
        </elementProp>
        <stringProp name="ThreadGroup.num_threads">{threads}</stringProp>
        <stringProp name="ThreadGroup.ramp_time">{ramp}</stringProp>
        <boolProp name="ThreadGroup.scheduler">false</boolProp>
        <stringProp name="ThreadGroup.duration"></stringProp>
        <stringProp name="ThreadGroup.delay"></stringProp>
        <boolProp name="ThreadGroup.same_user_on_next_iteration">true</boolProp>
      </ThreadGroup>
      <hashTree>
{samplers}        <ResultCollector guiclass="ViewResultsFullVisualizer" testclass="ResultCollector" testname="View Results Tree" enabled="true">
          <boolProp name="ResultCollector.error_logging">false</boolProp>
          <objProp>
            <name>saveConfig</name>
            <value class="SampleSaveConfiguration">
              <time>true</time>
              <latency>true</latency>
              <timestamp>true</timestamp>
              <success>true</success>
              <label>true</label>
              <code>true</code>
              <message>true</message>
              <threadName>true</threadName>
              <dataType>true</dataType>
              <encoding>false</encoding>
              <assertions>true</assertions>
              <subresults>true</subresults>
              <responseData>false</responseData>
              <samplerData>false</samplerData>
              <xml>false</xml>
              <fieldNames>true</fieldNames>
              <responseHeaders>false</responseHeaders>
              <requestHeaders>false</requestHeaders>
              <responseDataOnError>false</responseDataOnError>
              <saveAssertionResultsFailureMessage>true</saveAssertionResultsFailureMessage>
              <assertionsResultsToSave>0</assertionsResultsToSave>
              <bytes>true</bytes>
              <sentBytes>true</sentBytes>
              <url>true</url>
              <threadCounts>true</threadCounts>
              <idleTime>true</idleTime>
              <connectTime>true</connectTime>
            </value>
          </objProp>
          <stringProp name="filename"></stringProp>
        </ResultCollector>
        <hashTree/>
        <ResultCollector guiclass="SummaryReport" testclass="ResultCollector" testname="Summary Report" enabled="true">
          <boolProp name="ResultCollector.error_logging">false</boolProp>
          <objProp>
            <name>saveConfig</name>
            <value class="SampleSaveConfiguration">
              <time>true</time>
              <latency>true</latency>
              <timestamp>true</timestamp>
              <success>true</success>
              <label>true</label>
              <code>true</code>
              <message>true</message>
              <threadName>true</threadName>
              <dataType>true</dataType>
              <encoding>false</encoding>
              <assertions>true</assertions>
              <subresults>true</subresults>
              <responseData>false</responseData>
              <samplerData>false</samplerData>
              <xml>false</xml>
              <fieldNames>true</fieldNames>
              <responseHeaders>false</responseHeaders>
              <requestHeaders>false</requestHeaders>
              <responseDataOnError>false</responseDataOnError>
              <saveAssertionResultsFailureMessage>true</saveAssertionResultsFailureMessage>
              <assertionsResultsToSave>0</assertionsResultsToSave>
              <bytes>true</bytes>
              <sentBytes>true</sentBytes>
              <url>true</url>
              <threadCounts>true</threadCounts>
              <idleTime>true</idleTime>
              <connectTime>true</connectTime>
            </value>
          </objProp>
          <stringProp name="filename"></stringProp>
        </ResultCollector>
        <hashTree/>
      </hashTree>
    </hashTree>
  </hashTree>
</jmeterTestPlan>
"#,
        plan = xml_escape(plan_name),
        loops = loops,
        threads = threads,
        ramp = ramp_up,
        samplers = samplers,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::KeyValue;

    fn sample_collection() -> ExportCollection {
        ExportCollection {
            format: "apitest-collection".into(),
            version: 1,
            name: "Demo API".into(),
            base_url: "https://httpbin.org".into(),
            requests: vec![ExportRequest {
                name: "Get IP".into(),
                method: "GET".into(),
                url: "/ip".into(),
                params: vec![],
                headers: vec![KeyValue::text("Accept", "application/json")],
                body_type: "none".into(),
                body_content: String::new(),
                pre_script: String::new(),
                test_script: String::new(),
                mock_enabled: false,
                mock_status: 200,
                mock_headers: vec![],
                mock_body: String::new(),
                mock_delay_ms: 0,
            }],
        }
    }

    #[test]
    fn exports_valid_jmeter_xml() {
        let xml = collection_to_jmx(&sample_collection(), 5, 10, 1);
        assert!(xml.contains("jmeterTestPlan"));
        assert!(xml.contains("HTTPSamplerProxy"));
        assert!(xml.contains("Get IP"));
        assert!(xml.contains("httpbin.org"));
        assert!(xml.contains("<stringProp name=\"ThreadGroup.num_threads\">5</stringProp>"));
        assert!(xml.contains("HeaderManager"));
        assert!(xml.contains("Accept"));
    }

    #[test]
    fn escapes_xml_in_body() {
        let mut c = sample_collection();
        c.requests[0].method = "POST".into();
        c.requests[0].body_type = "json".into();
        c.requests[0].body_content = r#"{"a":"<b>&c"}"#.into();
        let xml = collection_to_jmx(&c, 1, 1, 0);
        assert!(xml.contains("&lt;b&gt;&amp;c"));
        assert!(!xml.contains(r#"{"a":"<b>&c"}"#));
    }
}
