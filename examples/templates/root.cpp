namespace {{namespace}} {

{% for node in ast.nodes -%}
  {% if node.trait == "struct" -%}
    struct {{node.name}} {
       {% for member in node.members -%}
         {{member.type.name}} {{member.name}};
       {% endfor -%}
    };
  {% elif node.trait == "enum" -%}
    enum {{node.name}} {
       {% for member in node.members -%}
         {{member.name}},
       {% endfor -%}
    };
  {% endif %}
{% endfor -%}

} // {{namespace}}
