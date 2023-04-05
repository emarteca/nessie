import javascript

class ArgExpr extends Expr {
	DataFlow::InvokeNode fct_call;

	ArgExpr() {
		fct_call.getAnArgument().asExpr() = this
	}

	DataFlow::InvokeNode getFctCall() {
		result = fct_call
	}
}

class LabelledLiteral extends Literal {
	string type_label;

	LabelledLiteral() {
		this instanceof NullLiteral and type_label = "null" 
		or
		this instanceof BooleanLiteral and type_label = "bool"
		or
		this instanceof NumberLiteral and type_label = "number"
		or 
		this instanceof BigIntLiteral and type_label = "bigint"
		or 
		this instanceof StringLiteral and type_label = "string" 
		or 
		this instanceof RegExpLiteral and type_label = "regex"
	}

	string getTypeLabel() {
		result = type_label
	}

	string getStringRep() {
		result = getRawValue()
	}
}

class OneDepthPrimitiveArray extends ArrayExpr {
	OneDepthPrimitiveArray() {
		forall(Expr elt | elt = this.getAnElement() | elt instanceof LabelledLiteral)
	}

	string getStringRep() {
		result = "[" + concat(LabelledLiteral elt, int pos 
			| elt = this.getElement(pos) 
			| elt.getValue(), "," order by pos) + "]"
	}
}

class OneDepthPrimitiveObj extends ObjectExpr {

	OneDepthPrimitiveObj() {
		forall(Property prop | prop = this.getAProperty() | prop.getInit() instanceof LabelledLiteral and exists(prop.getName()))
	}

	string getStringRep() {
		result = "{" + concat(Property prop, int pos 
			| prop = this.getProperty(pos) 
			| prop.getName() + ": " + prop.getInit().(LabelledLiteral).getStringRep(), ", " order by pos) + "}"
	}
}

// literals but also arrays and objects


class ConstantArg extends ArgExpr {
	string type_label;

	ConstantArg() {
		this instanceof OneDepthPrimitiveObj and type_label = "Object"
		or
		this instanceof OneDepthPrimitiveArray and type_label = "Array"
		or
		type_label = this.(LabelledLiteral).getTypeLabel()
	}

	string getTypeLabel() {
		result = type_label
	}

	string getStringRep() {
		result = this.(LabelledLiteral).getStringRep()
		or
		result = this.(OneDepthPrimitiveObj).getStringRep()
		or
		result = this.(OneDepthPrimitiveArray).getStringRep()
	}
}

class APICallWithSig extends API::Node {

	DataFlow::CallNode call;

	// condition: there's at least one constant arg
	APICallWithSig() {
		exists(ConstantArg arg | arg.getFctCall() = call and call = this.getACall())
	}

	string getSignature() {
		result = "(" + concat(ArgExpr arg, int pos | arg = call.getArgument(pos).asExpr() | getArgTypeRep(arg), "," order by pos) + ")"
	}

	string getArgTypeRep(ArgExpr arg) {
		result = arg.(ConstantArg).getTypeLabel()
		or
		arg instanceof OverFctExpr and result = "_FUNCTION_"
		or
		(not (arg instanceof ConstantArg or arg instanceof OverFctExpr)) and result = "_NOT_CONST_OR_FCT_"
	}

	string getArgRep(ArgExpr arg) {
		result = arg.(ConstantArg).getStringRep()
		or
		arg instanceof OverFctExpr and result = "_FUNCTION_"
		or
		(not (arg instanceof ConstantArg or arg instanceof OverFctExpr)) and result = "_NOT_CONST_OR_FCT_"
	}

	string getSigWithValues() {
		result = "(" + concat(ArgExpr arg, int pos | arg = call.getArgument(pos).asExpr() | getArgRep(arg), "," order by pos) + ")"
	}
}

class OverFctExpr extends Expr, Function {
	OverFctExpr() {
		this instanceof FunctionExpr 
		or
		this instanceof ArrowFunctionExpr
	}

	override Stmt getEnclosingStmt() {
		result = this.(ArrowFunctionExpr).getEnclosingStmt()
		or
		result = this.(FunctionExpr).getEnclosingStmt()
	}	
}

// from API::Node nd, ConstantArg arg
// where nd.getACall() = arg.getFctCall()
// select nd, arg.getFctCall().getCalleeName(), arg.getFctCall().getNumArgument(), arg, arg.getTypeLabel(), arg.getStringRep()

from APICallWithSig cs
select cs, cs.getSignature(), cs.getSigWithValues()