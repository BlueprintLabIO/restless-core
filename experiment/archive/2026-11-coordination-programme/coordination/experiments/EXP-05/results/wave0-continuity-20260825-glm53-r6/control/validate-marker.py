import json,sys
a=json.load(open(sys.argv[1])); b=json.load(open(sys.argv[2]))
assert b == a and set(b) == {'marker'}
print(json.dumps({'valid':True,'marker':b['marker']},sort_keys=True))
