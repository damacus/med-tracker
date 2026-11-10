{{- define "med-tracker.name" -}}
{{- .Values.fullnameOverride | default .Release.Name | trunc 42 | trimSuffix "-" -}}
{{- end -}}

{{- define "med-tracker.image" -}}
{{- $image := printf "%s:%s" .Values.image.repository (.Values.image.tag | default .Chart.AppVersion) -}}
{{- if .Values.image.digest -}}
{{- printf "%s@%s" $image .Values.image.digest -}}
{{- else -}}
{{- $image -}}
{{- end -}}
{{- end -}}

{{- define "med-tracker.runtime-secret" -}}
{{- .Values.database.runtime.existingSecret | default (printf "%s-runtime" (include "med-tracker.name" .)) -}}
{{- end -}}

{{- define "med-tracker.update-secret" -}}
{{- .Values.database.update.existingSecret | default (printf "%s-update" (include "med-tracker.name" .)) -}}
{{- end -}}
