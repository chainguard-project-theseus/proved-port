package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"sort"
)

// step is one step of a resolved check: its argv and its name.
type step struct {
	Cmd  []string `json:"cmd"`
	Name string   `json:"name"`
}

// job is a resolved check as `chainctl checks explain --output=json`
// prints it, with the fields a call rule reads decoded and the rest kept
// raw, so a call rule can refuse a check with a field it does not read.
type job struct {
	fields   map[string]json.RawMessage
	steps    []step
	services []service
	values   []value
}

// value is a variable's value, with where it is set (`variable NAME`).
type value struct {
	where, text string
}

// addValues adds the values of a variables object (absent when raw is
// nil), in name order, each named where + its name.
func (j *job) addValues(raw json.RawMessage, where string) error {
	if raw == nil {
		return nil
	}
	var vars map[string]string
	if err := json.Unmarshal(raw, &vars); err != nil {
		return err
	}
	names := make([]string, 0, len(vars))
	for k := range vars {
		names = append(names, k)
	}
	sort.Strings(names)
	for _, k := range names {
		j.values = append(j.values, value{where + k, vars[k]})
	}
	return nil
}

// workflow is the resolved configuration: its checks by name.
type workflow struct {
	jobs  map[string]*job
	names []string
}

// callFields are the check fields a call rule reads or knows not to
// change what a step runs. A call rule refuses a check with any other.
var callFields = map[string]bool{
	"advisory":    true,
	"egress":      true,
	"environment": true,
	"foreach":     true,
	"identity":    true,
	"network":     true,
	"resources":   true,
	"schedule":    true,
	"steps":       true,
	"timeout":     true,
	"trigger":     true,
	"triggers":    true,
	"variables":   true,
	"when":        true,
}

// strict decodes raw into v, refusing a field v does not have and any
// data after the value.
func strict(raw []byte, v any) error {
	d := json.NewDecoder(bytes.NewReader(raw))
	d.DisallowUnknownFields()
	if err := d.Decode(v); err != nil {
		return err
	}
	if _, err := d.Token(); err != io.EOF {
		return fmt.Errorf("data after the value")
	}
	return nil
}

// readWorkflow reads the JSON `chainctl checks explain --output=json`
// prints: one object whose only field is `jobs`, an object of checks,
// each with `steps`, a list of objects with only `cmd` (a list of
// strings, not empty) and `name`. Anything else is an error: a refused
// configuration prints its problems, not `jobs`.
func readWorkflow(data []byte) (*workflow, error) {
	var top map[string]json.RawMessage
	if err := strict(data, &top); err != nil {
		return nil, fmt.Errorf("not a resolved workflow (JSON): %v", err)
	}
	if top == nil {
		return nil, fmt.Errorf("not a resolved workflow: not a JSON object")
	}
	rawJobs, ok := top["jobs"]
	if !ok {
		return nil, fmt.Errorf("not a resolved workflow: no `jobs` (chainctl checks explain prints the problems instead when it refuses the configuration)")
	}
	for k := range top {
		if k != "jobs" {
			return nil, fmt.Errorf("the workflow has a field this tool does not read: %q", k)
		}
	}
	var jobs map[string]json.RawMessage
	if err := strict(rawJobs, &jobs); err != nil || jobs == nil {
		return nil, fmt.Errorf("`jobs` is not an object of checks")
	}
	// explain prints `{}` for a configuration with no check; a forbid
	// rule would hold over it vacuously.
	if len(jobs) == 0 {
		return nil, fmt.Errorf("`jobs` holds no check")
	}
	w := &workflow{jobs: map[string]*job{}}
	for name := range jobs {
		w.names = append(w.names, name)
	}
	sort.Strings(w.names)
	for _, name := range w.names {
		var fields map[string]json.RawMessage
		if err := strict(jobs[name], &fields); err != nil || fields == nil {
			return nil, fmt.Errorf("check %s is not an object", name)
		}
		j := &job{fields: fields}
		rawSteps, ok := fields["steps"]
		if !ok {
			return nil, fmt.Errorf("check %s has no steps", name)
		}
		if err := strict(rawSteps, &j.steps); err != nil {
			return nil, fmt.Errorf("check %s: steps: %v", name, err)
		}
		for k, s := range j.steps {
			if len(s.Cmd) == 0 {
				return nil, fmt.Errorf("check %s: step %d has no cmd", name, k+1)
			}
		}
		// A forbid rule reads the services' commands and the variables'
		// values too, so they are read in every check that has them.
		if rawEnv, ok := fields["environment"]; ok {
			var env map[string]json.RawMessage
			if err := strict(rawEnv, &env); err != nil || env == nil {
				return nil, fmt.Errorf("check %s: environment is not an object", name)
			}
			if rawSvc, ok := env["services"]; ok {
				if err := strict(rawSvc, &j.services); err != nil {
					return nil, fmt.Errorf("check %s: environment services: %v", name, err)
				}
			}
			if err := j.addValues(env["variables"], "environment variable "); err != nil {
				return nil, fmt.Errorf("check %s: environment variables: %v", name, err)
			}
		}
		if err := j.addValues(fields["variables"], "variable "); err != nil {
			return nil, fmt.Errorf("check %s: variables: %v", name, err)
		}
		if rawEach, ok := fields["foreach"]; ok {
			var each struct {
				Entries []struct {
					Name string          `json:"name"`
					Vars json.RawMessage `json:"vars"`
				} `json:"entries"`
			}
			if err := json.Unmarshal(rawEach, &each); err != nil {
				return nil, fmt.Errorf("check %s: foreach: %v", name, err)
			}
			for _, e := range each.Entries {
				if err := j.addValues(e.Vars, "foreach entry "+e.Name+" var "); err != nil {
					return nil, fmt.Errorf("check %s: foreach entry %s vars: %v", name, e.Name, err)
				}
			}
		}
		w.jobs[name] = j
	}
	return w, nil
}

// service is a sidecar a check's environment starts: a command that runs
// beside the steps, in workdir.
type service struct {
	Name    string   `json:"name"`
	Cmd     []string `json:"cmd"`
	Timeout string   `json:"timeout"`
	Workdir string   `json:"workdir"`
}

// environment is a check's resolved environment. Its services run
// beside the steps and do not change what a step runs.
type environment struct {
	Packages  []string          `json:"packages"`
	Variables map[string]string `json:"variables"`
	Services  []service         `json:"services"`
}

// when is a check's resolved condition.
type when struct {
	Requested bool     `json:"requested"`
	Changed   []string `json:"changed"`
}

// foreach is a check's resolved instances.
type foreach struct {
	Entries []struct {
		Name      string            `json:"name"`
		Vars      map[string]string `json:"vars"`
		Resources string            `json:"resources"`
		Network   json.RawMessage   `json:"network"`
		Timeout   string            `json:"timeout"`
	} `json:"entries"`
}

// field decodes the check's field name into v, strictly; a field the
// check lacks leaves v as it is.
func (j *job) field(name string, v any) error {
	raw, ok := j.fields[name]
	if !ok {
		return nil
	}
	if err := strict(raw, v); err != nil {
		return fmt.Errorf("its %s field: %v", name, err)
	}
	return nil
}
