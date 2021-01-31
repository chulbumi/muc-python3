# -*- coding: utf-8 -*-

"""
/*  HanLP의 감정 표현 부분을 누구맘대로? 제니퍼 맘대로~ 가져왔다.
 *
 *                          99/04/12 AM 01.30 MaGuN
 *
 *  HanLP를 위해서 새로만든 부분입니다.
 *  string 변수로 다음과 같이 사용할 수 있습니다.
 *
 *
 *      자신인경우          대상이 있을 경우.
 *
 *      $N - 자신           대상    this_player
 *      $I - 당신(은)       이름(이)    this_player
 *      $S - 당신(이)       이름(이)    this_player
 *      $J - 당신(을)       이름(을)    this_player
 *
 *      $T - 대상           대상    target
 *      $D - 당신(은)       대상(은)    target
 *      $O - 자신(을)       대상(을)    target
 *      $A - 자신(과)       대상(과)    target
 *
 *      $M - 사용자가 입력한 변수를 출력 ( 없을땐 기본적인 modifier를 출력 )
 */
"""
from lib.object import *
from include.path import *
from lib.comm import broadcast

class Emotes:

    emotes = []

    def create(self):
        self.load()

    def save(self):
        pass

    def load(self):
        pass

    def get_emotes(self):
        return self.emotes.keys()

    def get_emote(self, verb):
        return self.emotes[verb]

    def delete_emote(self, verb):
        del self.emotes[verb]
        self.save()

    def add_emote(self, verb, line):
        if len(verb) == 0 or len(line) == 0:
            return 0
        self.emotes[verb] = line
        return 1

    def tell_emote(self, line, owner, target = None, special = '')
        
        if len(line) == 0 or line[0] == '#':
            return 0

        name = owner.get('이름')

        if target == None:
            tname = name
        else:
            tname = target.get('이름')

        env = owner.env
        if env == None:
            return 0

        import re
        m = re.match('(\S+)에게 (:(\S+):)', line)

        if m != None and len(m.groups()) != 2:
            m = re.match('(\S+) (:(\S+):)', line)

        if m == None return 0

        try:
            modifier = str(m.groups()[-1])
            word = list(m.groups()[:-1])
            tword = list(m.groups()[:-1])
            eword = list(m.groups()[:-1])
        except:
            return 0
        word.append('\n')
        tword.append('\n')
        eword.append('\n')

        for (i=0; i < sz; i++) {
	        if( sscanf(word[i], "%s$M%s", tmp1, tmp2) == 2 ) {
                if (special)
		        word[i] = tword[i] = eword[i] = tmp1+special+tmp2;
	            else if(modifier)
		        word[i] = tword[i] = eword[i] = tmp1+modifier+tmp2;
	            else
		        word[i] = tword[i] = eword[i] = tmp1+""+tmp2;
	        }
	        else if( sscanf(word[i], "%s$N%s", tmp1, tmp2) == 2 ) {
	            word[i] = tmp1+"당신"+tmp2;
	            tword[i] = eword[i] = tmp1+name+tmp2;
	        }
	        else if( sscanf(word[i], "%s$T%s", tmp1, tmp2) == 2 ) {
	            if( owner == target ) {
		            word[i] = tword[i] = eword[i] = tmp1+"자신"+tmp2;
	            }
	            else {
		            tword[i] = tmp1+"당신"+tmp2;
		            word[i] = eword[i] = tmp1+tname+tmp2;
	            }
	        }   
	        else {
	            switch(word[i]) {
		        case "$I":
		            word[i] = "당신은";
		            tword[i] = eword[i] = name + han_iga(name);
		            break;
		        case "$S":
		            word[i] = "당신이";
		            tword[i] = eword[i] = name + han_iga(name);
		            break;
		        case "$J":
		            word[i] = "당신을";
		            tword[i] = eword[i] = name + han_obj(name);
		            break;
		        case "$O":
		            if( owner == target ) {
			            word[i] = tword[i] = eword[i] = "자신을";
    		        }
	    	        else {
                        tword[i] = "당신을";
    			        word[i] = eword[i] = tname + han_obj(tname);
	    	        }
	    	        break;
	    	    case "$A":
	    	        if( owner == target ) {
	    		        word[i] = tword[i] = eword[i] = "자신과";
	    	        }
	    	        else {
	    		        tword[i] = "당신과";
	    		        word[i] = eword[i] = name + han_and(tname);
	    	        }
	    	        break;
    		    case "$D":
    		        if( owner == target ) {
    			        word[i] = "당신은";
    			        tword[i] = eword[i] = han_desc(name);
    		        }
    		        else {
    			        tword[i] = "당신은";
    			        word[i] = eword[i] = han_desc(tname);
    		        }
    		        break;
    	        }
    	    }
        }

        if( target && target != owner ) {
	        word = implode(word, " ");
        	tword = implode(tword, " ");
	        eword = implode(eword, " ");
	        message("tell_emote", word, owner);
       	    message("tell_emote", tword, target);
	        message("tell_emote", eword, env, ({  owner, target }) );
	        return 1;
        }
        word = implode(word, " ");
        eword = implode(eword, " ");
        message("tell_emote", word, owner);
        message("tell_emote", eword, env, ({  owner }) );
        return 1;
    }

    def parse(string command, string str )
        string name, etc, *emotion;
        object owner, target, env;

        emotion = emotes[command];
        if( !emotion ) return 0;

        owner = this_player();
        env = environment( owner );

        if( !str ) return tell_emote(emotion[0]);
        if( sscanf( str,"%s %s", name, etc ) == 2 ) {
	        target = present( name, env );
	        if(!target) return tell_emote( emotion[0], owner, str );
	        else return tell_emote( emotion[1], target, etc );
        }
        target = present( str, env );
        if(!target) return tell_emote( emotion[0], owner, str );
        return tell_emote( emotion[1], target, etc );
